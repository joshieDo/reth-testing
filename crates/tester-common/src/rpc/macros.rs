#[macro_export]
macro_rules! rpc {
    ($self:expr, $method:ident $(, $args:expr )* ) => {
        Box::pin($self.test_rpc_call(
            stringify!($method),
            move |client: &C|  {
                client.$method( $( $args.clone(), )*)
            }
        )) as Pin<Box<dyn Future<Output = ($crate::rpc::MethodName, Result<(), $crate::rpc::TestError>)> + Send>>
    };
}
