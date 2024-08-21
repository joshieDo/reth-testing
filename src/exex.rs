use crate::{engine::advance_chain, etherscan::etherscan_provider, rpc::TesterStatus};
use parking_lot::RwLock;
use reth::{
    api::FullNodeComponents,
    primitives::BlockNumberOrTag,
    providers::{BlockIdReader, BlockNumReader, StageCheckpointReader},
    rpc::builder::auth::AuthServerHandle,
};
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_stages_types::StageId;
use reth_tracing::tracing::info;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Uses etherscan to move the chain forward **until** it has collected `max_blocks`.
pub async fn exex<Node: FullNodeComponents>(
    mut ctx: ExExContext<Node>,
    auth_server_receiver: oneshot::Receiver<AuthServerHandle>,
    rpc_status: Arc<RwLock<TesterStatus>>,
    etherscan: Option<String>,
    max_blocks: u64,
) -> eyre::Result<()> {
    let auth_client = auth_server_receiver.await?.http_client();
    let finalized = ctx.provider().finalized_block_num_hash()?.unwrap_or_default();
    let etherscan = etherscan_provider(&ctx, etherscan)?;
    let initial_height = ctx.provider().last_block_number()?;
    
    let mut local_tip = initial_height;
    let mut etherscan_tip =
        etherscan.load_block(BlockNumberOrTag::Latest).await?.header.number.unwrap_or_default();
    
    rpc_status.write().initial_height = initial_height;

    info!(local_tip, etherscan_tip, ?finalized, "Starting exex.");
    loop {
        // StageId::Bodies gets updated on each flush to disk
        let storage_tip =
            ctx.provider().get_stage_checkpoint(StageId::Bodies)?.unwrap_or_default().block_number;

        // Updates the `tester/status`
        {
            let mut rpc_status_rw = rpc_status.write();
            rpc_status_rw.in_memory_first = storage_tip + 1;
            rpc_status_rw.tip = local_tip;
        }

        // Have reached the maximum number of blocks so we can exit the exex
        if local_tip >= initial_height + max_blocks {
            info!(
                etherscan_tip,
                local_tip, storage_tip, initial_height, "Stopped moving chain forward"
            );

            // Updates the `tester/status`
            rpc_status.write().ready = true;

            return Ok(())
        }

        // Query the next block
        if local_tip < etherscan_tip {
            info!(etherscan_tip, local_tip, storage_tip, "Advancing chain");

            local_tip += 1;
            advance_chain(&auth_client, &etherscan, local_tip, finalized.hash).await?;

            if let Some(notification) = ctx.notifications.recv().await {
                match &notification {
                    ExExNotification::ChainCommitted { new } => {
                        info!(committed_chain = ?new.range(), "Received commit");
                    }
                    ExExNotification::ChainReorged { old, new } => {
                        info!(from_chain = ?old.range(), to_chain = ?new.range(), "Received reorg");
                    }
                    ExExNotification::ChainReverted { old } => {
                        info!(reverted_chain = ?old.range(), "Received revert");
                    }
                };

                if let Some(committed_chain) = notification.committed_chain() {
                    ctx.events.send(ExExEvent::FinishedHeight(committed_chain.tip().number))?;
                }
            }
        }

        // Avoids hitting rate limits
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        etherscan_tip =
            etherscan.load_block(BlockNumberOrTag::Latest).await?.header.number.unwrap_or_default()
    }
}
