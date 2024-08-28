use super::{MethodName, ReportResults, TestError};
use console::Style;
use jsonrpsee::http_client::HttpClient;
use reth_tracing::tracing::trace;
use serde::Serialize;
use similar::{ChangeTag, TextDiff};
use std::fmt::Debug;

/// Prints test results to console presenting a coloured diff.
pub(crate) fn report(results_by_block: ReportResults) {
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

/// Compares the response to a specific method between a local and remote node.
pub(crate) async fn test_method<'a, F, Fut, T, E>(
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

#[macro_export]
macro_rules! test_eth_rpc_method {
    ($rpc_pair:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin($crate::rpc::utils::test_method(
            stringify!($method),
            $rpc_pair,
            move |client: &HttpClient|  {
                reth::rpc::api::EthApiClient::<Transaction, Block>::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = ($crate::rpc::MethodName, Result<(), $crate::rpc::TestError>)> + Send>>
    };
}

#[macro_export]
macro_rules! test_filter_eth_rpc_method {
    ($rpc_pair:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin($crate::rpc::utils::test_method(
            stringify!($method),
            $rpc_pair,
            move |client: &HttpClient|  {
                reth::rpc::api::EthFilterApiClient::<Transaction>::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = ($crate::rpc::MethodName, Result<(), $crate::rpc::TestError>)> + Send>>
    };
}

#[macro_export]
macro_rules! test_reth_rpc_method {
    ($rpc_pair:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin($crate::rpc::utils::test_method(
            stringify!($method),
            $rpc_pair,
            move |client: &HttpClient|  {
                reth::rpc::api::RethApiClient::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = ($crate::rpc::MethodName, Result<(), $crate::rpc::TestError>)> + Send>>
    };
}

#[macro_export]
macro_rules! test_debug_rpc_method {
    ($rpc_pair:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin($crate::rpc::utils::test_method(
            stringify!($method),
            $rpc_pair,
            move |client: &HttpClient|  {
                reth::rpc::api::DebugApiClient::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = ($crate::rpc::MethodName, Result<(), $crate::rpc::TestError>)> + Send>>
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! test_trace_rpc_method {
    ($rpc_pair:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin($crate::rpc::utils::test_method(
            stringify!($method),
            $rpc_pair,
            move |client: &HttpClient|  {
                reth::rpc::api::TraceApiClient::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = ($crate::rpc::MethodName, Result<(), $crate::rpc::TestError>)> + Send>>
    };
}
