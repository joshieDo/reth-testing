#![cfg_attr(not(test), warn(unused_crate_dependencies))]
use clap::Parser;
use reth::{builder::EngineNodeLauncher, cli::Cli, providers::providers::BlockchainProvider2};
use reth_engine_tree::tree::TreeConfig;
use reth_node_ethereum::{node::EthereumAddOns, EthereumNode};

mod equality;

mod exex;
use exex::exex;

mod engine;
mod etherscan;

mod rpc;
use rpc::{TesterExt, TesterExtApiServer};

fn default_persistence() -> u64 {
    TreeConfig::default().persistence_threshold()
}

#[derive(Debug, clap::Parser)]
#[command(next_help_heading = "Testing ExEx")]
pub struct TestArgs {
    #[arg(long, value_name = "ETHERSCAN_API_URL")]
    pub etherscan_url: Option<String>,
    /// Uses etherscan to sync up to `num_blocks`. **Should not** be used with a CL.
    #[arg(long, default_value_t = default_persistence())]
    pub num_blocks: u64,
    /// Runs equality tests across many RPCs calls after syncing `num_blocks`.
    #[arg(long)]
    pub against_rpc: Option<String>,
}

fn main() {
    let (engine_api_handle_tx, engine_api_handle_rx) = tokio::sync::oneshot::channel();

    let rpc_ext = TesterExt::new();
    let rpc_status = rpc_ext.watcher.clone();

    Cli::<TestArgs>::parse()
        .run(|builder, args| async move {
            let handle = builder
                .with_types_and_provider::<EthereumNode, BlockchainProvider2<_>>()
                .with_components(EthereumNode::components())
                .with_add_ons::<EthereumAddOns>()
                .extend_rpc_modules(move |ctx| {
                    ctx.modules.merge_configured(rpc_ext.into_rpc())?;
                    Ok(())
                })
                .on_rpc_started(|_ctx, handles| {
                    let _ = engine_api_handle_tx.send((handles.auth.clone(), handles.rpc.clone()));
                    Ok(())
                })
                .install_exex("tester", move |ctx| async move {
                    Ok(exex(ctx, engine_api_handle_rx, rpc_status, args))
                })
                .launch_with_fn(|builder| {
                    let launcher = EngineNodeLauncher::new(
                        builder.task_executor().clone(),
                        builder.config().datadir(),
                    );
                    builder.launch_with(launcher)
                })
                .await?;

            handle.wait_for_node_exit().await
        })
        .unwrap();
}
