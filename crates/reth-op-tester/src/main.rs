#![cfg_attr(not(test), warn(unused_crate_dependencies))]
use clap::Parser;
use reth::{builder::EngineNodeLauncher, providers::providers::BlockchainProvider2};
use reth_optimism_cli::{chainspec::OpChainSpecParser, Cli};
use reth_optimism_node::{node::OpAddOns, OpNode};
use tester_common::node::{
    args::TestArgs,
    exex::exex,
    ext::{TesterExt, TesterExtApiServer},
};

fn main() {
    let (engine_api_handle_tx, engine_api_handle_rx) = tokio::sync::oneshot::channel();

    let rpc_ext = TesterExt::new();
    let rpc_status = rpc_ext.watcher.clone();

    Cli::<OpChainSpecParser, TestArgs>::parse()
        .run(|builder, args| async move {
            let handle = builder
                .with_types_and_provider::<OpNode, BlockchainProvider2<_>>()
                .with_components(OpNode::components(Default::default()))
                .with_add_ons(OpAddOns::default())
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
                        Default::default(),
                    );
                    builder.launch_with(launcher)
                })
                .await?;

            handle.wait_for_node_exit().await
        })
        .unwrap();
}
