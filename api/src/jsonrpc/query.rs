use std::sync::Arc;

use crate::{
    error::ApiError,
    jsonrpc::{AccountHistoryRpcServer, AxonStatusRpcServer},
};
use common::types::{
    api::{
        AddressAmount, ChainState, HistoryEvent, OperationType, Pagination, PaginationResult,
        RewardHistory, RewardState, StakeAmount, StakeRate, StakeState,
    },
    relation_db::transaction_history,
    smt::Address,
};

use jsonrpsee::core::{async_trait, RpcResult};
use storage::relation_db::RelationDB;

pub struct StatusRpcModule {
    storage: Arc<RelationDB>,
}

impl StatusRpcModule {
    pub fn new(storage: Arc<RelationDB>) -> Self {
        Self { storage }
    }
}

#[async_trait]
impl AccountHistoryRpcServer for StatusRpcModule {
    async fn get_stake_rate(&self, addr: Address) -> RpcResult<StakeRate> {
        let res = self
            .storage
            .get_address_state(addr)
            .await
            .map_err(ApiError::from)?;

        if res.stake_amount == 0 && res.delegate_amount == 0 {
            return Ok(StakeRate {
                address:       addr,
                stake_rate:    f64::default(),
                delegate_rate: f64::default(),
            });
        }

        let sum = (res.stake_amount + res.delegate_amount) as f64;
        let stake_rate = res.stake_amount as f64 / sum;
        let delegate_rate = res.delegate_amount as f64 / sum;

        Ok(StakeRate {
            address: addr,
            stake_rate,
            delegate_rate,
        })
    }

    async fn get_stake_state(&self, addr: Address) -> RpcResult<StakeState> {
        let res = self
            .storage
            .get_address_state(addr)
            .await
            .map_err(ApiError::from)?;

        Ok(StakeState {
            total_amount:        (res.stake_amount + res.delegate_amount + res.withdrawable_amount)
                as u64,
            stake_amount:        res.stake_amount as u64,
            delegate_amount:     res.delegate_amount as u64,
            withdrawable_amount: res.withdrawable_amount as u64,
        })
    }

    async fn get_reward_state(&self, addr: Address) -> RpcResult<RewardState> {
        let res = self
            .storage
            .get_address_state(addr)
            .await
            .map_err(ApiError::from)?;

        Ok(RewardState {
            lock_amount:   res.reward_lock_amount as u64,
            unlock_amount: res.reward_unlock_amount as u64,
        })
    }

    async fn get_stake_history(
        &self,
        addr: Address,
        pagination: Pagination,
        event: HistoryEvent,
    ) -> RpcResult<PaginationResult<transaction_history::Model>> {
        let res = self
            .storage
            .get_operation_history(
                addr,
                OperationType::Stake.into(),
                event.into(),
                pagination.offset(),
                pagination.limit(),
            )
            .await
            .map_err(ApiError::from)?;

        Ok(PaginationResult::new(res))
    }

    async fn get_reward_history(
        &self,
        addr: Address,
        pagination: Pagination,
    ) -> RpcResult<PaginationResult<RewardHistory>> {
        let res = self
            .storage
            .get_reward_history(addr, pagination.offset(), pagination.limit())
            .await
            .map_err(ApiError::from)?;

        Ok(PaginationResult::new(res))
    }

    async fn get_stake_amount_by_epoch(
        &self,
        epoch: u64,
        operation_type: OperationType,
    ) -> RpcResult<StakeAmount> {
        let res = self
            .storage
            .get_amount_by_epoch(epoch, operation_type.into())
            .await
            .map_err(ApiError::from)?;

        Ok(res)
    }

    async fn get_top_stake_address(&self, limit: u64) -> RpcResult<Vec<AddressAmount>> {
        let res = self
            .storage
            .get_top_stake_address(limit)
            .await
            .map_err(ApiError::from)?;

        Ok(res
            .iter()
            .map(|r| AddressAmount {
                address: r.address.clone(),
                amount:  r.stake_amount as u64,
            })
            .collect())
    }

    async fn get_latest_stake_transactions(
        &self,
        pagination: Pagination,
    ) -> RpcResult<PaginationResult<transaction_history::Model>> {
        let res = self
            .storage
            .get_latest_stake_transactions(pagination.offset(), pagination.limit())
            .await
            .map_err(ApiError::from)?;

        Ok(PaginationResult::new(res))
    }
}

#[derive(Default)]
pub struct AxonStatusRpc {}

impl AxonStatusRpc {
    pub fn new() -> Self {
        AxonStatusRpc {}
    }
}

#[async_trait]
impl AxonStatusRpcServer for AxonStatusRpc {
    async fn get_chain_state(&self) -> RpcResult<ChainState> {
        let res = ChainState::default();
        Ok(res)
    }
}
