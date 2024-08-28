use super::{MethodName, TestError};
use crate::{
    rpc::report::report, test_debug_rpc_method, test_eth_rpc_method, test_filter_eth_rpc_method,
    test_reth_rpc_method, test_trace_rpc_method,
};
use eyre::Result;
use futures::Future;
use jsonrpsee::http_client::HttpClient;
use reth::{
    primitives::{BlockId, BlockNumber, BlockNumberOrTag},
    rpc::{
        api::EthApiClient,
        types::{
            trace::geth::{
                GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions,
            },
            Block, Filter, Index, Transaction,
        },
    },
};
use reth_tracing::tracing::info;
use std::{collections::BTreeMap, ops::RangeInclusive, pin::Pin};

// Alias type
type BlockTestResults = BTreeMap<BlockNumber, Vec<(MethodName, Result<(), TestError>)>>;

/// RpcTester
#[derive(Debug)]
pub struct RpcTester {
    /// First RPC node.
    rpc1: HttpClient,
    /// Second RPC node.
    rpc2: HttpClient,
    /// Source of truth. This may be one of the rpcs above for convenience reasons.
    truth: HttpClient,
    /// Whether to query tracing methods
    use_tracing: bool,
    /// Whether to query reth namespace
    use_reth: bool,
}

impl RpcTester {
    /// Returns [`Self`].
    pub fn new(rpc1: HttpClient, rpc2: HttpClient) -> Self {
        let truth = rpc2.clone();
        Self { use_tracing: true, use_reth: true, rpc1, rpc2, truth }
    }

    /// Adds [`HttpClient`] as source of truth.
    pub fn with_truth(mut self, truth: HttpClient) -> Self {
        self.truth = truth;
        self
    }

    /// Disables tracing calls.
    pub fn without_tracing(mut self) -> Self {
        self.use_tracing = true;
        self
    }

    /// Disables reth namespace.
    pub fn without_reth(mut self) -> Self {
        self.use_reth = true;
        self
    }

    /// Verifies that a suite of RPC calls matches the results of a remote node.
    pub async fn test_equality(&self, block_range: RangeInclusive<BlockNumber>) -> Result<()> {
        self.test_per_block(block_range.clone()).await?;
        self.test_block_range(block_range).await?;
        Ok(())
    }

    /// Verifies RPC calls applicable to single blocks.
    async fn test_per_block(&self, block_range: RangeInclusive<u64>) -> Result<(), eyre::Error> {
        let mut results = BlockTestResults::new();
        let rpc_pair = (&self.rpc1, &self.rpc2);

        for block_number in block_range {
            info!("# test rpc {block_number}");

            let mut tests = vec![];

            let block_or_tag = BlockNumberOrTag::Number(block_number);
            let block_id = BlockId::Number(block_or_tag);
            let block: Block = EthApiClient::<Transaction, Block>::block_by_number(
                &self.truth,
                block_number.into(),
                true,
            )
            .await?
            .expect("should have block from range");
            assert_eq!(block.header.number.expect("should have number"), block_number);
            let block_hash = block.header.hash.expect("block range should not include pending");

            // Block based
            #[rustfmt::skip]
            tests.extend(vec![
                test_eth_rpc_method!(rpc_pair, block_by_hash, block_hash, true),
                test_eth_rpc_method!(rpc_pair, block_by_number, block_or_tag, true),
                test_eth_rpc_method!(rpc_pair, block_transaction_count_by_hash, block_hash),
                test_eth_rpc_method!(rpc_pair, block_transaction_count_by_number, block_or_tag),
                test_eth_rpc_method!(rpc_pair, block_uncles_count_by_hash, block_hash),
                test_eth_rpc_method!(rpc_pair, block_uncles_count_by_number, block_or_tag),
                test_eth_rpc_method!(rpc_pair, block_receipts, block_id),
                test_eth_rpc_method!(rpc_pair, header_by_number, block_or_tag),
                test_eth_rpc_method!(rpc_pair, header_by_hash, block_hash),
                test_reth_rpc_method!(rpc_pair, reth_get_balance_changes_in_block, block_id),
                // Response is too big & Http(TooLarge))
                // test_debug_rpc_method!(rpc_pair, debug_trace_block_by_number, block_or_tag, None)
                test_trace_rpc_method!(rpc_pair, trace_block, block_id),
                test_filter_eth_rpc_method!(rpc_pair, logs, Filter::new().select(block_number)),
            ]);

            // // Transaction/Receipt based RPCs
            for (index, tx) in block.transactions.into_transactions().enumerate() {
                let tracer_opts = Some(GethDebugTracingOptions::default().with_tracer(
                    GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::CallTracer),
                ));

                if let Some(receipt) =
                    EthApiClient::<Transaction, Block>::transaction_receipt(&self.truth, tx.hash)
                        .await?
                {
                    if let Some(log) = receipt.inner.inner.logs().first().cloned() {
                        #[rustfmt::skip]
                        tests.push(
                            test_filter_eth_rpc_method!(rpc_pair, logs, Filter::new().select(block_number).address(log.address()))
                        );
                    }

                    if let Some(topic) = receipt
                        .inner
                        .inner
                        .logs()
                        .last()
                        .and_then(|log| log.data().topics().first())
                        .cloned()
                    {
                        #[rustfmt::skip]
                        tests.push(
                            test_filter_eth_rpc_method!(rpc_pair, logs, Filter::new().select(block_number).event_signature(topic))
                        );
                    }
                }

                let index: Index = index.into();

                #[rustfmt::skip]
                tests.extend(vec![
                    test_eth_rpc_method!(rpc_pair, raw_transaction_by_hash, tx.hash),
                    test_eth_rpc_method!(rpc_pair, transaction_by_hash, tx.hash),
                    test_eth_rpc_method!(rpc_pair, raw_transaction_by_block_hash_and_index, block_hash,index),
                    test_eth_rpc_method!(rpc_pair, transaction_by_block_hash_and_index, block_hash, index),
                    test_eth_rpc_method!(rpc_pair, raw_transaction_by_block_number_and_index, block_or_tag, index ),
                    test_eth_rpc_method!(rpc_pair, transaction_by_block_number_and_index, block_or_tag, index ),
                    test_eth_rpc_method!(rpc_pair, transaction_receipt, tx.hash),
                    test_eth_rpc_method!(rpc_pair, transaction_count, tx.from, Some(block_id)),
                    test_eth_rpc_method!(rpc_pair, balance, tx.from, Some(block_id)),
                    test_debug_rpc_method!(rpc_pair, debug_trace_transaction, tx.hash, tracer_opts)
                ]);
            }
            let block_results = futures::future::join_all(tests).await;
            results.insert(block_number, block_results);
        }
        report(results.into_iter().map(|(k, v)| (format!("Block Number {k}"), v)).collect());
        Ok(())
    }

    /// Verifies RPC calls applicable to block ranges.
    async fn test_block_range(&self, block_range: RangeInclusive<u64>) -> Result<(), eyre::Error> {
        let start = *block_range.start();
        let end = *block_range.end();
        let rpc_pair = (&self.rpc1, &self.rpc2);

        #[rustfmt::skip]
        report(vec![(
            format!("{}..={}", start, end),
            futures::future::join_all([
                test_filter_eth_rpc_method!(rpc_pair, logs, Filter::new().from_block(start).to_block(end)
            )])
            .await,
        )]);

        Ok(())
    }
}
