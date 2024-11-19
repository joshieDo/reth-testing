use alloy_primitives::BlockNumber;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use parking_lot::RwLock;
use serde::Serialize;
use std::sync::Arc;

/// trait interface for a custom rpc namespace: `tester`
#[rpc(server, namespace = "tester")]
pub trait TesterExtApi {
    /// Returns [`TesterStatus`]
    #[method(name = "status")]
    fn status(&self) -> RpcResult<TesterStatus>;
}

/// The type that implements the `Tester` rpc namespace trait
#[allow(dead_code)]
#[derive(Debug, Serialize, Clone, Default)]
pub struct TesterStatus {
    /// Whether it has stopped advancing the chain forward
    pub ready: bool,
    /// Initial block number
    pub initial_height: BlockNumber,
    /// Latest block number
    pub tip: BlockNumber,
    /// First block number in-memory.
    pub in_memory_first: BlockNumber,
}

/// The type that implements the `Tester` rpc namespace trait
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TesterExt {
    pub watcher: Arc<RwLock<TesterStatus>>,
}

impl TesterExt {
    pub fn new() -> Self {
        Self { watcher: Arc::new(RwLock::new(TesterStatus::default())) }
    }
}

impl TesterExtApiServer for TesterExt {
    fn status(&self) -> RpcResult<TesterStatus> {
        Ok(self.watcher.read().clone())
    }
}
