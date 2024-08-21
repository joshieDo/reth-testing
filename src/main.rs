#![cfg_attr(not(test), warn(unused_crate_dependencies))]

use clap::Parser;
use jsonrpsee::http_client::{transport::HttpBackend, HttpClient};
use reth::{
    api::FullNodeComponents,
    cli::Cli,
    primitives::{BlockHash, BlockNumberOrTag},
    providers::{BlockIdReader, BlockNumReader, StageCheckpointReader},
    rpc::{api::EngineApiClient, builder::auth::AuthServerHandle, types::engine::ForkchoiceState},
};
use reth_consensus_debug_client::{rich_block_to_execution_payload_v3, EtherscanBlockProvider};
use reth_engine_tree::tree::TreeConfig;
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_node_ethereum::{EthEngineTypes, EthereumNode};
use reth_rpc_layer::AuthClientService;
use reth_stages_types::StageId;
use reth_tracing::tracing::{info, warn};
use tokio::sync::oneshot;

/// Uses etherscan to move the chain forward **until** it has collected `max_blocks`.
async fn exex<Node: FullNodeComponents>(
    mut ctx: ExExContext<Node>,
    auth_server_receiver: oneshot::Receiver<AuthServerHandle>,
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

    info!(local_tip, etherscan_tip, ?finalized, "Starting exex.");

    loop {
        let storage_tip =
            ctx.provider().get_stage_checkpoint(StageId::Bodies)?.unwrap_or_default().block_number;

        if local_tip >= initial_height + max_blocks {
            info!(
                etherscan_tip,
                local_tip, storage_tip, initial_height, "Stopped moving chain forward"
            );
            return Ok(())
        }

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
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        etherscan_tip =
            etherscan.load_block(BlockNumberOrTag::Latest).await?.header.number.unwrap_or_default()
    }
}

/// Advances the chain by querying `etherscan` for a specific block and issues a `newPayload` &
/// `FCU` request from that.
async fn advance_chain(
    auth_client: &HttpClient<AuthClientService<HttpBackend>>,
    etherscan: &EtherscanBlockProvider,
    block_number: u64,
    finalized_hash: BlockHash,
) -> Result<(), eyre::Error> {
    let etherscan_block = etherscan.load_block(block_number.into()).await?;
    let payload = rich_block_to_execution_payload_v3(etherscan_block);
    let block_hash = payload.block_hash();

    EngineApiClient::<EthEngineTypes>::new_payload_v3(
        auth_client,
        payload.execution_payload_v3,
        payload.versioned_hashes,
        payload.parent_beacon_block_root,
    )
    .await
    .inspect_err(|err|  {
        warn!(target: "exex-consensus", %err, %block_hash,  %block_number, "failed to submit new payload to execution client");
    })?;

    EngineApiClient::<EthEngineTypes>::fork_choice_updated_v3(
        auth_client,
        ForkchoiceState {
            head_block_hash: block_hash,
            safe_block_hash: finalized_hash,
            finalized_block_hash: finalized_hash,
        },
        None,
    )
    .await
    .inspect_err(|err| {
        warn!(target: "exex-consensus", %err, "failed to submit fork
            choice update to execution client");
    })?;

    Ok(())
}

fn etherscan_provider<Node: FullNodeComponents>(
    ctx: &ExExContext<Node>,
    etherscan: Option<String>,
) -> Result<EtherscanBlockProvider, eyre::Error> {
    let chain = ctx.config.chain.chain;
    let etherscan_url = etherscan.map(Ok).unwrap_or_else(|| {
        chain
            .etherscan_urls()
            .map(|urls| urls.0.to_string())
            .ok_or_else(|| eyre::eyre!("failed to get etherscan url for chain: {chain}"))
    })?;
    let etherscan_api_key = chain.etherscan_api_key().ok_or_else(|| {
        eyre::eyre!("etherscan api key not found for rpc consensus client for chain: {chain}")
    })?;
    Ok(EtherscanBlockProvider::new(etherscan_url, etherscan_api_key))
}

fn default_persistence() -> u64 {
    TreeConfig::default().persistence_threshold()
}

#[derive(Debug, clap::Parser)]
#[command(next_help_heading = "Testing ExEx")]
pub struct TestArgs {
    #[arg(long, default_value_t = default_persistence())]
    pub num_blocks: u64,
    #[arg(long, value_name = "ETHERSCAN_API_URL")]
    pub etherscan_url: Option<String>,
}

fn main() {
    let (tx, rx) = tokio::sync::oneshot::channel();
    Cli::<TestArgs>::parse()
        .run(|builder, args| async move {
            let handle = builder
                .node(EthereumNode::default())
                .on_rpc_started(|_ctx, handles| {
                    let _ = tx.send(handles.auth.clone());
                    Ok(())
                })
                .install_exex("tester", move |ctx| async move {
                    Ok(exex(ctx, rx, args.etherscan_url, args.num_blocks))
                })
                .launch()
                .await?;

            handle.wait_for_node_exit().await
        })
        .unwrap();
}
