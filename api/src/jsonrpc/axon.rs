use crate::jsonrpc::AxonStatusRpcServer;
use common::{traits::async_trait, types::api::ChainState};
use jsonrpsee::core::RpcResult;

pub struct AxonStatusRpc {}

impl AxonStatusRpc {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl AxonStatusRpcServer for AxonStatusRpc {
    async fn get_chain_state(&self) -> RpcResult<ChainState> {
        unimplemented!()
    }
}
