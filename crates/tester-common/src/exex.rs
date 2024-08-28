use crate::{
    args::TestArgs,
    rpc::{equality::test_rpc_equality, ext::TesterStatus},
};
use fake_cl::FakeCl;
use parking_lot::RwLock;
use reth::{
    api::FullNodeComponents,
    primitives::BlockNumberOrTag,
    providers::{BlockIdReader, BlockNumReader, StageCheckpointReader},
    rpc::builder::{auth::AuthServerHandle, RpcServerHandle},
};
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_stages_types::StageId;
use reth_tracing::tracing::info;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Uses etherscan to move the chain forward **until** it has collected `num_blocks`.
pub async fn exex<Node: FullNodeComponents>(
    mut ctx: ExExContext<Node>,
    server_receiver: oneshot::Receiver<(AuthServerHandle, RpcServerHandle)>,
    rpc_status: Arc<RwLock<TesterStatus>>,
    args: TestArgs,
) -> eyre::Result<()> {
    let TestArgs { etherscan_url, num_blocks, against_rpc } = args;
    let (auth_handle, rpc_handle) = server_receiver.await?;
    let auth_client = auth_handle.http_client();
    let finalized = ctx.provider().finalized_block_num_hash()?.unwrap_or_default();
    let mut fake_cl = FakeCl::new(ctx.config.chain.chain, etherscan_url)?;
    let initial_height = ctx.provider().last_block_number()?;

    let mut local_tip = initial_height;
    let mut etherscan_tip = fake_cl
        .etherscan
        .load_block(BlockNumberOrTag::Latest)
        .await?
        .header
        .number
        .unwrap_or_default();

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
        if local_tip >= initial_height + num_blocks {
            info!(
                etherscan_tip,
                local_tip, storage_tip, initial_height, "Stopped moving chain forward"
            );

            // Updates the `tester/status` with ready
            rpc_status.write().ready = true;

            if let Some(url) = against_rpc {
                test_rpc_equality(
                    ctx,
                    &url,
                    rpc_handle.http_client().expect("should have rpc"),
                    (storage_tip - 2)..=local_tip,
                )
                .await?;
            }

            // Exiting would crash the node, so we sleep forever instead.
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(500)).await
            }
        }

        // Query the next block
        if local_tip < etherscan_tip {
            info!(etherscan_tip, local_tip, storage_tip, "Advancing chain");

            local_tip += 1;
            fake_cl.advance_chain(&auth_client, local_tip, finalized.hash).await?;

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

        etherscan_tip = fake_cl
            .etherscan
            .load_block(BlockNumberOrTag::Latest)
            .await?
            .header
            .number
            .unwrap_or_default()
    }
}
