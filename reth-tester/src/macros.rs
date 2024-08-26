#[macro_export]
macro_rules! test_eth_rpc_method {
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

#[macro_export]
macro_rules! test_filter_eth_rpc_method {
    ($rpc_pair:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin(test_method(
            stringify!($method),
            $rpc_pair,
            move |client: &HttpClient|  {
                EthFilterApiClient::<Transaction>::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = (MethodName, Result<(), TestError>)> + Send>>
    };
}

#[macro_export]
macro_rules! test_reth_rpc_method {
    ($rpc_pair:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin(test_method(
            stringify!($method),
            $rpc_pair,
            move |client: &HttpClient|  {
                RethApiClient::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = (MethodName, Result<(), TestError>)> + Send>>
    };
}

#[macro_export]
macro_rules! test_debug_rpc_method {
    ($rpc_pair:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin(test_method(
            stringify!($method),
            $rpc_pair,
            move |client: &HttpClient|  {
                DebugApiClient::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = (MethodName, Result<(), TestError>)> + Send>>
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! test_trace_rpc_method {
    ($rpc_pair:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin(test_method(
            stringify!($method),
            $rpc_pair,
            move |client: &HttpClient|  {
                TraceApiClient::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = (MethodName, Result<(), TestError>)> + Send>>
    };
}
