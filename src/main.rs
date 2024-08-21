#![cfg_attr(not(test), warn(unused_crate_dependencies))]
use clap::Parser;
use reth::cli::Cli;
use reth_engine_tree::tree::TreeConfig;
use reth_node_ethereum::EthereumNode;

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
    #[arg(long, default_value_t = default_persistence())]
    pub num_blocks: u64,
    #[arg(long, value_name = "ETHERSCAN_API_URL")]
    pub etherscan_url: Option<String>,
}

fn main() {
    let (engine_api_handle_tx, engine_api_handle_rx) = tokio::sync::oneshot::channel();

    let rpc_ext = TesterExt::new();
    let rpc_status = rpc_ext.watcher.clone();

    Cli::<TestArgs>::parse()
        .run(|builder, args| async move {
            let handle = builder
                .node(EthereumNode::default())
                .extend_rpc_modules(move |ctx| {
                    ctx.modules.merge_configured(rpc_ext.into_rpc())?;
                    Ok(())
                })
                .on_rpc_started(|_ctx, handles| {
                    let _ = engine_api_handle_tx.send(handles.auth.clone());
                    Ok(())
                })
                .install_exex("tester", move |ctx| async move {
                    Ok(exex(
                        ctx,
                        engine_api_handle_rx,
                        rpc_status,
                        args.etherscan_url,
                        args.num_blocks,
                    ))
                })
                .launch()
                .await?;

            handle.wait_for_node_exit().await
        })
        .unwrap();
}
