use super::{MethodName, TestError};
use crate::{
    rpc::report::report, test_debug_rpc_method, test_eth_rpc_method, test_filter_eth_rpc_method,
    test_reth_rpc_method, test_trace_rpc_method,
};
use eyre::Result;
use futures::Future;
use jsonrpsee::http_client::HttpClient;
use reth::{
    api::FullNodeComponents,
    primitives::{BlockId, BlockNumber, BlockNumberOrTag},
    providers::{BlockReader, ReceiptProvider},
    rpc::types::{
        trace::geth::{GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions},
        Block, Filter, Index, Transaction,
    },
};
use reth_exex::ExExContext;
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
        let truth = rpc1.clone();
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
}

/// Verifies that a suite of RPC calls matches the results of a remote node.
pub async fn test_rpc_equality<Node: FullNodeComponents>(
    ctx: ExExContext<Node>,
    local_rpc: &HttpClient,
    remote_rpc: &HttpClient,
    block_range: RangeInclusive<BlockNumber>,
) -> Result<()> {
    test_per_block(local_rpc, remote_rpc, block_range.clone(), &ctx).await?;
    test_block_range(local_rpc, remote_rpc, block_range).await?;
    Ok(())
}

/// Verifies RPC calls applicable to block ranges.
async fn test_block_range(
    local_rpc: &HttpClient,
    remote_rpc: &HttpClient,
    block_range: RangeInclusive<u64>,
) -> Result<(), eyre::Error> {
    let start = *block_range.start();
    let end = *block_range.end();
    let rpc_pair = (local_rpc, remote_rpc);

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

/// Verifies RPC calls applicable to single blocks.
async fn test_per_block<Node: FullNodeComponents>(
    local_rpc: &HttpClient,
    remote_rpc: &HttpClient,
    block_range: RangeInclusive<u64>,
    ctx: &ExExContext<Node>,
) -> Result<(), eyre::Error> {
    let mut results = BlockTestResults::new();
    let rpc_pair = (local_rpc, remote_rpc);

    // TODO: change to self.truth

    for block_number in block_range {
        info!("# test rpc {block_number}");

        let mut tests = vec![];
        let provider = ctx.provider();

        let block_or_tag = BlockNumberOrTag::Number(block_number);
        let block_id = BlockId::Number(block_or_tag);
        let block = provider.block(block_number.into())?.expect("block should exist");
        assert_eq!(block.number, block_number);
        let first_tx_num = provider
            .block_body_indices(block_number)?
            .expect("should have body indices")
            .first_tx_num;

        let block_hash = block.hash_slow();

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

        // Transaction/Receipt based RPCs
        for (index, tx) in block.body.iter().enumerate() {
            let tracer_opts = Some(GethDebugTracingOptions::default().with_tracer(
                GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::CallTracer),
            ));
            let receipt =
                provider.receipt(first_tx_num + index as u64)?.expect("should have receipt");
            let index: Index = index.into();
            let tx_hash = tx.hash;
            let signer = tx.recover_signer().expect("should recover sender");

            if let Some(log) = receipt.logs.first().cloned() {
                #[rustfmt::skip]
                tests.push(
                    test_filter_eth_rpc_method!(rpc_pair, logs, Filter::new().select(block_number).address(log.address))
                );
            }

            if let Some(topic) =
                receipt.logs.last().and_then(|log| log.data.topics().first()).cloned()
            {
                #[rustfmt::skip]
                tests.push(
                    test_filter_eth_rpc_method!(rpc_pair, logs, Filter::new().select(block_number).event_signature(topic))
                );
            }

            #[rustfmt::skip]
            tests.extend(vec![
                test_eth_rpc_method!(rpc_pair, raw_transaction_by_hash, tx_hash),
                test_eth_rpc_method!(rpc_pair, transaction_by_hash, tx_hash),
                test_eth_rpc_method!(rpc_pair, raw_transaction_by_block_hash_and_index, block_hash,index),
                test_eth_rpc_method!(rpc_pair, transaction_by_block_hash_and_index, block_hash, index),
                test_eth_rpc_method!(rpc_pair, raw_transaction_by_block_number_and_index, block_or_tag, index ),
                test_eth_rpc_method!(rpc_pair, transaction_by_block_number_and_index, block_or_tag, index ),
                test_eth_rpc_method!(rpc_pair, transaction_receipt, tx_hash),
                test_eth_rpc_method!(rpc_pair, transaction_count, signer, Some(block_id)),
                test_eth_rpc_method!(rpc_pair, balance, signer, Some(block_id)),
                test_debug_rpc_method!(rpc_pair, debug_trace_transaction, tx_hash, tracer_opts)
            ]);
        }
        let block_results = futures::future::join_all(tests).await;
        results.insert(block_number, block_results);
    }
    report(results.into_iter().map(|(k, v)| (format!("Block Number {k}"), v)).collect());
    Ok(())
}
