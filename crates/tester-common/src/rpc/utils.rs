use super::{MethodName, TestError};
use jsonrpsee::http_client::HttpClient;
use reth_tracing::tracing::trace;
use serde::Serialize;
use std::fmt::Debug;

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
