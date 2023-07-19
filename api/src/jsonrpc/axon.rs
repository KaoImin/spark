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
        Ok(ChainState {
            epoch:              0,
            period:             0,
            block_number:       0,
            total_stake_amount: 0,
        })
    }
}
