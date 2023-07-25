pub mod axon;
pub mod operation;
pub mod query;
use crate::error::ApiError;
use crate::jsonrpc::operation::OperationRpc;
use crate::jsonrpc::query::{AxonStatusRpc, StatusRpcModule};

use ckb_jsonrpc_types::TransactionView;
use common::types::api::{
    AddressAmount, ChainState, DelegateItem, DelegateRequirement, OperationType, Pagination,
    PaginationResult, RewardHistory, RewardState, RpcDelegateDeltas, StakeAmount, StakeRate,
    StakeState,
};
use common::types::{
    delta::DelegateDeltas, relation_db::transaction_history, smt::Address, H160, H256,
};
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::server::{ServerBuilder, ServerHandle};
use rpc_client::ckb_client::ckb_rpc_client::CkbRpcClient;
use storage::{RelationDB, KVDB};
use tokio::net::ToSocketAddrs;

use std::sync::atomic::AtomicU64;
use std::sync::Arc;

#[rpc(server)]
pub trait AccountHistoryRpc {
    #[method(name = "getStakeRate")]
    async fn get_stake_rate(&self, addr: Address) -> RpcResult<StakeRate>;

    #[method(name = "getStakeState")]
    async fn get_stake_state(&self, addr: Address) -> RpcResult<StakeState>;

    #[method(name = "getRewardState")]
    async fn get_reward_state(&self, addr: Address) -> RpcResult<RewardState>;

    #[method(name = "getStakeHistory")]
    async fn get_stake_history(
        &self,
        addr: Address,
        event: Option<u32>,
        pagination: Pagination,
    ) -> RpcResult<PaginationResult<transaction_history::Model>>;

    #[method(name = "getDelegateHistory")]
    async fn get_delegate_history(
        &self,
        addr: Address,
        event: Option<u32>,
        pagination: Pagination,
    ) -> RpcResult<PaginationResult<transaction_history::Model>>;

    #[method(name = "getRewardHistory")]
    async fn get_reward_history(
        &self,
        addr: Address,
        pagination: Pagination,
    ) -> RpcResult<PaginationResult<RewardHistory>>;

    #[method(name = "getStakeAmountByEpoch")]
    async fn get_stake_amount_by_epoch(
        &self,
        start_epoch: u64,
        end_epoch: u64,
        operation: u32,
    ) -> RpcResult<Vec<StakeAmount>>;

    #[method(name = "getTopStakeAddress")]
    async fn get_top_stake_address(&self, limit: u64) -> RpcResult<Vec<AddressAmount>>;

    #[method(name = "getLatestStakeTransactions")]
    async fn get_latest_stake_transactions(
        &self,
        pagination: Pagination,
    ) -> RpcResult<PaginationResult<transaction_history::Model>>;

    #[method(name = "getDelegateRecords")]
    async fn get_delegate_records(&self, addr: Address) -> RpcResult<RpcDelegateDeltas>;

    #[method(name = "getDelegateRequirement")]
    async fn get_delegate_requirement(&self, staker: Address) -> RpcResult<DelegateRequirement>;
}

#[rpc(server)]
pub trait AxonStatusRpc {
    #[method(name = "getChainState")]
    async fn get_chain_state(&self) -> RpcResult<ChainState>;
}

#[rpc(server)]
pub trait OperationRpc {
    #[method(name = "setStakeRate")]
    async fn set_stake_rate(
        &self,
        address: H256,
        stake_rate: u64,
        delegate_rate: u64,
    ) -> RpcResult<String>;

    #[method(name = "stake")]
    async fn stake(&self, address: H160, amount: u64) -> RpcResult<TransactionView>;

    #[method(name = "unstake")]
    async fn unstake(&self, address: H160, amount: u64) -> RpcResult<TransactionView>;

    #[method(name = "delegate")]
    async fn delegate(&self, address: H160, infos: Vec<DelegateItem>)
        -> RpcResult<TransactionView>;

    #[method(name = "undelegate")]
    async fn undelegate(
        &self,
        address: H160,
        infos: Vec<DelegateItem>,
    ) -> RpcResult<TransactionView>;

    #[method(name = "withdrawStake")]
    async fn withdraw_stake(
        &self,
        address: H160,
        withdraw_type: OperationType,
    ) -> RpcResult<TransactionView>;

    #[method(name = "withdrawRewards")]
    async fn withdraw_rewards(&self, address: H160) -> RpcResult<TransactionView>;

    #[method(name = "sendTransaction")]
    async fn send_transaction(&self, tx: TransactionView) -> RpcResult<ckb_types::H256>;
}

pub async fn run_server(
    storage: Arc<RelationDB>,
    kvdb: Arc<KVDB>,
    ckb_client: Arc<CkbRpcClient>,
    current_epoch: Arc<AtomicU64>,
    url: impl ToSocketAddrs,
) -> Result<ServerHandle, ApiError> {
    let mut module = StatusRpcModule::new(
        Arc::clone(&storage),
        kvdb,
        Arc::clone(&ckb_client),
        Arc::clone(&current_epoch),
    )
    .into_rpc();
    let axon_rpc = AxonStatusRpc::new().into_rpc();
    let op_rpc = OperationRpc::new(ckb_client, current_epoch).into_rpc();
    module.merge(axon_rpc).unwrap();
    module.merge(op_rpc).unwrap();

    let server = ServerBuilder::new()
        .http_only()
        .build(url)
        .await
        .map_err(|e| ApiError::HttpServer(e.to_string()))?;
    log::info!("RPC server listening: {:?}", server.local_addr().unwrap());

    Ok(server.start(module).unwrap())
}
