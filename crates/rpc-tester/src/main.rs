use clap::Parser;
use jsonrpsee::http_client::HttpClientBuilder;
use tester_common::rpc::equality::RpcTester;

type BlockNumber = u64;

#[derive(Debug, Parser)]
#[command(
    about = "Shows a diff of RPC results between two nodes over a series of calls within a block range"
)]
pub struct CliArgs {
    /// RPC URL 1
    #[arg(long, value_name = "RPC_URL1")]
    pub rpc1: String,

    /// RPC URL 2
    #[arg(long, value_name = "RPC_URL2")]
    pub rpc2: String,

    /// Starting block number
    #[arg(long, value_name = "BLOCK_START")]
    pub from: BlockNumber,

    /// Ending block number
    #[arg(long, value_name = "BLOCK_END_INCLUSIVE")]
    pub to: BlockNumber,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = CliArgs::parse();

    let rpc_tester = RpcTester::new(HttpClientBuilder::default().build(&args.rpc1)?, HttpClientBuilder::default().build(&args.rpc2)?);
    rpc_tester.test_equality(args.from..=args.to).await?;

    Ok(())
}
