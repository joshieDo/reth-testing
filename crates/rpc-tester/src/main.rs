use alloy_rpc_types::{Block, Receipt, SyncStatus, Transaction};
use clap::Parser;
use jsonrpsee::{http_client::HttpClientBuilder, tracing::info};
use std::{ops::RangeInclusive, thread::sleep, time::Duration};
// use reth_primitives::{Block, Receipt, Transaction};
use reth_rpc_api::EthApiClient;
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

    /// Number of blocks to test from the tip.
    #[arg(long, value_name = "NUM_BLOCKS", default_value = "256")]
    pub num_blocks: BlockNumber,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = CliArgs::parse();

    let rpc1 = HttpClientBuilder::default().build(&args.rpc1)?;
    let rpc2 = HttpClientBuilder::default().build(&args.rpc2)?;

    let block_range = wait_for_readiness(&rpc1, &rpc2, args.num_blocks).await?;

    RpcTester::new(rpc1, rpc2).test_equality(block_range).await
}

/// Waits until rpc1 is synced to the tip and returns a valid block range to test against rpc2.
pub async fn wait_for_readiness<C>(
    rpc1: &C,
    rpc2: &C,
    block_size_range: u64,
) -> eyre::Result<RangeInclusive<BlockNumber>>
where
    C: EthApiClient<Transaction, Block, Receipt> + Clone + Send + Sync,
{
    let sleep = || sleep(Duration::from_secs(5));

    // Waits until it's done syncing
    while let SyncStatus::Info(sync_info) = rpc1.syncing().await? {
        info!("rpc1 still syncing: {sync_info:?}");
        sleep();
    }

    // Waits until rpc1 has _mostly_ catch up to rpc2 or beyond
    loop {
        let tip1: u64 = rpc1.block_number().await?.try_into()?;
        let tip2: u64 = rpc2.block_number().await?.try_into()?;

        if tip1 >= tip2 || tip2 - tip1 <= 5 {
            return Ok(tip2 - block_size_range..=tip2)
        }

        sleep();
    }
}
