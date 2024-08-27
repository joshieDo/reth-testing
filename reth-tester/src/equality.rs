use crate::{
    test_eth_rpc_method, test_filter_eth_rpc_method, test_reth_rpc_method, test_trace_rpc_method,
};
use console::Style;
use eyre::Result;
use futures::Future;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use reth::{
    api::FullNodeComponents,
    primitives::{BlockId, BlockNumber, BlockNumberOrTag},
    providers::{BlockReader, ReceiptProvider},
    rpc::{
        api::{EthApiClient, EthFilterApiClient, RethApiClient, TraceApiClient},
        types::{Block, Filter, Index, Transaction},
    },
};
use reth_exex::ExExContext;
use reth_tracing::tracing::{info, trace};
use serde::Serialize;
use similar::{ChangeTag, TextDiff};
use std::{collections::BTreeMap, fmt::Debug, ops::RangeInclusive, pin::Pin};

/// Verifies that a suite of RPC calls matches the results of a remote node.
pub async fn test_rpc_equality<Node: FullNodeComponents>(
    ctx: ExExContext<Node>,
    remote_rpc_url: &str,
    local_rpc: HttpClient,
    block_range: RangeInclusive<BlockNumber>,
) -> Result<()> {
    let remote_rpc = HttpClientBuilder::default().build(remote_rpc_url)?;
    test_per_block(&local_rpc, &remote_rpc, block_range.clone(), &ctx).await?;
    test_block_range(&local_rpc, &remote_rpc, block_range).await?;
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
                // test_debug_rpc_method!(rpc_pair, debug_trace_transaction, tx_hash, None)
            ]);
        }
        let block_results = futures::future::join_all(tests).await;
        results.insert(block_number, block_results);
    }
    report(results.into_iter().map(|(k, v)| (format!("Block Number {k}"), v)).collect());
    Ok(())
}

fn report(results_by_block: ReportResults) {
    println!("\n--- RPC Method Test Results ---");

    for (title, results) in results_by_block {
        let failures: Vec<_> = results
            .into_iter()
            .filter_map(|(name, result)| result.err().map(|err| (name, err)))
            .collect();

        if failures.is_empty() {
            println!("{title} ✅");
        } else {
            println!("\n{title} ❌");
            for (name, err) in failures {
                println!("    {name}: ❌ Failure ");
                match err {
                    TestError::Diff { local, remote } => {
                        let diff = TextDiff::from_lines(&local, &remote);
                        for op in diff.ops() {
                            for change in diff.iter_changes(op).peekable() {
                                let (sign, style) = match change.tag() {
                                    ChangeTag::Delete => ("-", Style::new().red()),
                                    ChangeTag::Insert => ("+", Style::new().green()),
                                    ChangeTag::Equal => (" ", Style::new()),
                                };
                                print!("{}{}", style.apply_to(sign).bold(), style.apply_to(change));
                            }
                        }
                    }
                    TestError::LocalErr(err) => println!("## Local node error: {err}"),
                    TestError::RemoteErr(err) => println!("## Remote node error: {err}"),
                }
            }
        }
    }

    println!("--------------------------------\n");
}

async fn test_method<'a, F, Fut, T, E>(
    name: &str,
    rpc_pair: (&'a HttpClient, &'a HttpClient),
    method_call: F,
) -> (MethodName, Result<(), TestError>)
where
    F: Fn(&'a HttpClient) -> Fut + 'a,
    Fut: std::future::Future<Output = Result<T, E>> + 'a + Send,
    T: PartialEq + Debug + Serialize,
    E: Debug,
{
    trace!("## {name}");

    let (local_result, remote_result) =
        tokio::join!(method_call(rpc_pair.0), method_call(rpc_pair.1));

    let result = match (local_result, remote_result) {
        (Ok(local), Ok(remote)) => {
            if local == remote {
                Ok(())
            } else {
                Err(TestError::Diff {
                    local: serde_json::to_string_pretty(&local).expect("should json"),
                    remote: serde_json::to_string_pretty(&remote).expect("should json"),
                })
            }
        }
        (Err(e), _) => Err(TestError::LocalErr(format!("{e:?}"))),
        (Ok(_), Err(e)) => Err(TestError::RemoteErr(format!("{e:?}"))),
    };

    (name.to_string(), result)
}

enum TestError {
    Diff { local: String, remote: String },
    LocalErr(String),
    RemoteErr(String),
}

type BlockTestResults = BTreeMap<BlockNumber, Vec<(MethodName, Result<(), TestError>)>>;
type ReportResults = Vec<(String, Vec<(MethodName, Result<(), TestError>)>)>;
type MethodName = String;
