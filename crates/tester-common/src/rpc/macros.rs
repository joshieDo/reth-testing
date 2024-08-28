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
