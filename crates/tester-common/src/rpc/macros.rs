#[macro_export]
macro_rules! test_eth_rpc_method {
    ($self:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin($self.test_rpc_call(
            stringify!($method),
            move |client: &HttpClient|  {
                reth::rpc::api::EthApiClient::<Transaction, Block, Receipt>::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = ($crate::rpc::MethodName, Result<(), $crate::rpc::TestError>)> + Send>>
    };
}

#[macro_export]
macro_rules! test_filter_eth_rpc_method {
    ($self:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin($self.test_rpc_call(
            stringify!($method),
            move |client: &HttpClient|  {
                reth::rpc::api::EthFilterApiClient::<Transaction>::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = ($crate::rpc::MethodName, Result<(), $crate::rpc::TestError>)> + Send>>
    };
}

#[macro_export]
macro_rules! test_reth_rpc_method {
    ($self:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin($self.test_rpc_call(
            stringify!($method),
            move |client: &HttpClient|  {
                reth::rpc::api::RethApiClient::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = ($crate::rpc::MethodName, Result<(), $crate::rpc::TestError>)> + Send>>
    };
}

#[macro_export]
macro_rules! test_debug_rpc_method {
    ($self:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin($self.test_rpc_call(
            stringify!($method),
            move |client: &HttpClient|  {
                reth::rpc::api::DebugApiClient::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = ($crate::rpc::MethodName, Result<(), $crate::rpc::TestError>)> + Send>>
    };
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! test_trace_rpc_method {
    ($self:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin($self.test_rpc_call(
            stringify!($method),
            move |client: &HttpClient|  {
                reth::rpc::api::TraceApiClient::$method(client $(, $args.clone() )*)
            }
        )) as Pin<Box<dyn Future<Output = ($crate::rpc::MethodName, Result<(), $crate::rpc::TestError>)> + Send>>
    };
}
