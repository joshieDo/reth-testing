use reth_engine_tree::tree::TreeConfig;

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
