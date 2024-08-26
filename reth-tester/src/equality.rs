use eyre::Result;
use futures::Future;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use reth::{
    api::FullNodeComponents,
    primitives::{BlockId, BlockNumber, BlockNumberOrTag},
    providers::BlockReader,
    rpc::{
        api::EthApiClient,
        types::{Block, Index, Transaction},
    },
};
use reth_exex::ExExContext;
use std::{collections::BTreeMap, fmt::Debug, ops::RangeInclusive, pin::Pin};

macro_rules! test_rpc_method {
    ($rpc_pair:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin(test_method(
            stringify!($method),
            $rpc_pair,
            move |client: &HttpClient|  {
                EthApiClient::<Transaction, Block>::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = (MethodName, Result<(), TestError>)> + Send>>
    };
}

/// Verifies that a suite of RPC calls matches the results of a remote node.
pub async fn test_rpc_equality<Node: FullNodeComponents>(
    ctx: ExExContext<Node>,
    remote_rpc_url: &str,
    local_rpc: HttpClient,
    block_range: RangeInclusive<BlockNumber>,
) -> Result<()> {
    let mut results = TestSuiteResults::new();

    let remote_rpc = HttpClientBuilder::default().build(remote_rpc_url)?;
    let rpc_pair = (&local_rpc, &remote_rpc);

    for block_number in block_range {
        let mut tests = vec![];
        let block_or_tag = BlockNumberOrTag::Number(block_number);
        let block_id = BlockId::Number(block_or_tag);
        let provider = ctx.provider();

        let block = provider.block(block_number.into())?.expect("block should exist");
        assert_eq!(block.number, block_number);

        let block_hash = block.hash_slow();
        tests.extend(vec![
            test_rpc_method!(rpc_pair, block_by_hash, block_hash, true),
            test_rpc_method!(rpc_pair, block_by_number, block_or_tag, true),
            test_rpc_method!(rpc_pair, block_transaction_count_by_hash, block_hash),
            test_rpc_method!(rpc_pair, block_transaction_count_by_number, block_or_tag),
            test_rpc_method!(rpc_pair, block_uncles_count_by_hash, block_hash),
            test_rpc_method!(rpc_pair, block_uncles_count_by_number, block_or_tag),
            test_rpc_method!(rpc_pair, block_receipts, block_id),
            test_rpc_method!(rpc_pair, header_by_number, block_or_tag),
            test_rpc_method!(rpc_pair, header_by_hash, block_hash),
        ]);

        for (index, tx) in block.body.into_iter().enumerate() {
            let index: Index = index.into();
            let tx_hash = tx.hash;
            let tx = tx.try_into_ecrecovered().expect("should recover sender");
            let signer = tx.signer();

            tests.extend(vec![
                test_rpc_method!(rpc_pair, raw_transaction_by_hash, tx_hash),
                test_rpc_method!(rpc_pair, transaction_by_hash, tx_hash),
                test_rpc_method!(
                    rpc_pair,
                    raw_transaction_by_block_hash_and_index,
                    block_hash,
                    index
                ),
                test_rpc_method!(rpc_pair, transaction_by_block_hash_and_index, block_hash, index),
                test_rpc_method!(
                    rpc_pair,
                    raw_transaction_by_block_number_and_index,
                    block_or_tag,
                    index
                ),
                test_rpc_method!(
                    rpc_pair,
                    transaction_by_block_number_and_index,
                    block_or_tag,
                    index
                ),
                test_rpc_method!(rpc_pair, transaction_receipt, tx_hash),
                test_rpc_method!(rpc_pair, transaction_count, signer, Some(block_id)),
                test_rpc_method!(rpc_pair, balance, signer, Some(block_id)),
            ]);
        }
        let block_results = futures::future::join_all(tests).await;
        results.insert(block_number, block_results);
    }

    report(results);

    Ok(())
}

fn report(results_by_block: TestSuiteResults) {
    println!("\n--- RPC Method Test Results by Block ---");

    for (block_number, results) in results_by_block {
        let failures: Vec<_> = results
            .into_iter()
            .filter_map(|(name, result)| result.err().map(|err| (name, err)))
            .collect();

        if failures.is_empty() {
            println!("Block Number: {block_number} ✅");
        } else {
            println!("\nBlock Number: {block_number} ❌");
            for (name, err) in failures {
                println!("    {name}: ❌ Failure {err:?}");
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
    T: PartialEq + Debug,
    E: Debug,
{
    let (local_result, remote_result) =
        tokio::join!(method_call(rpc_pair.0), method_call(rpc_pair.1));

    let result = match (local_result, remote_result) {
        (Ok(local), Ok(remote)) => {
            if local == remote {
                Ok(())
            } else {
                Err(TestError::Diff { local: format!("{local:?}"), remote: format!("{remote:?}") })
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

impl Debug for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestError::Diff { local, remote } => {
                write!(f, "\n### Diff ###\n# Local: {local}\n\n# Remote: {remote}")
            }
            TestError::LocalErr(err) => write!(f, "## Local node error: {err}"),
            TestError::RemoteErr(err) => write!(f, "## Remote node error: {err}"),
        }
    }
}

type TestSuiteResults = BTreeMap<BlockNumber, Vec<(MethodName, Result<(), TestError>)>>;
type MethodName = String;
