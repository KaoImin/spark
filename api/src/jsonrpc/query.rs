use std::sync::{atomic::AtomicU64, Arc};

use crate::{
    error::ApiError,
    jsonrpc::{AccountHistoryRpcServer, AxonStatusRpcServer},
};
use common::{
    types::{
        api::{
            AddressAmount, ChainState, DelegateRequirement, OperationType, Pagination,
            PaginationResult, RewardHistory, RewardState, RpcDelegateDeltas, StakeAmount,
            StakeRate, StakeState,
        },
        axon_types::delegate::DelegateCellData,
        delta::DelegateDeltas,
        relation_db::{total_amount, transaction_history},
        smt::Address,
    },
    utils::convert::{to_ckb_h160, to_u128, to_u32, to_u8},
};

use jsonrpsee::core::{async_trait, RpcResult};
use molecule::prelude::*;
use rpc_client::ckb_client::ckb_rpc_client::CkbRpcClient;
use storage::{relation_db::RelationDB, KVDB};
use tx_builder::ckb::{
    helper::{Delegate, Stake},
    METADATA_TYPE_ID, XUDT_OWNER,
};

pub struct StatusRpcModule {
    storage:    Arc<RelationDB>,
    kvdb:       Arc<KVDB>,
    ckb_client: Arc<CkbRpcClient>,

    current_epoch: Arc<AtomicU64>,
}

impl StatusRpcModule {
    pub fn new(
        storage: Arc<RelationDB>,
        kvdb: Arc<KVDB>,
        ckb_client: Arc<CkbRpcClient>,
        current_epoch: Arc<AtomicU64>,
    ) -> Self {
        Self {
            storage,
            kvdb,
            ckb_client,
            current_epoch,
        }
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

        if res.is_none() {
            return Ok(StakeRate {
                address:       to_ckb_h160(&addr),
                stake_rate:    f64::default(),
                delegate_rate: f64::default(),
            });
        }

        let res = res.unwrap();

        if res.stake_amount == 0 && res.delegate_amount == 0 {
            return Ok(StakeRate {
                address:       to_ckb_h160(&addr),
                stake_rate:    f64::default(),
                delegate_rate: f64::default(),
            });
        }

        let sum = (res.stake_amount + res.delegate_amount) as f64;
        let stake_rate = res.stake_amount as f64 / sum;
        let delegate_rate = res.delegate_amount as f64 / sum;

        Ok(StakeRate {
            address: to_ckb_h160(&addr),
            stake_rate,
            delegate_rate,
        })
    }

    async fn get_stake_state(&self, addr: Address) -> RpcResult<StakeState> {
        let res = self
            .storage
            .get_address_state(addr)
            .await
            .map_err(ApiError::from)?
            .unwrap_or(total_amount::Model {
                address:              addr.to_string(),
                stake_amount:         0,
                delegate_amount:      0,
                withdrawable_amount:  0,
                reward_unlock_amount: 0,
                reward_lock_amount:   0,
            });

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
            .map_err(ApiError::from)?
            .unwrap_or(total_amount::Model {
                address:              addr.to_string(),
                stake_amount:         0,
                delegate_amount:      0,
                withdrawable_amount:  0,
                reward_unlock_amount: 0,
                reward_lock_amount:   0,
            });

        Ok(RewardState {
            lock_amount:   res.reward_lock_amount as u64,
            unlock_amount: res.reward_unlock_amount as u64,
        })
    }

    async fn get_stake_history(
        &self,
        addr: Address,
        event: Option<u32>,
        pagination: Pagination,
    ) -> RpcResult<PaginationResult<transaction_history::Model>> {
        let res = self
            .storage
            .get_operation_history(
                addr,
                OperationType::Stake.into(),
                event,
                pagination.offset(),
                pagination.limit(),
            )
            .await
            .map_err(ApiError::from)?;

        Ok(PaginationResult::new(res))
    }

    async fn get_delegate_history(
        &self,
        addr: Address,
        event: Option<u32>,
        pagination: Pagination,
    ) -> RpcResult<PaginationResult<transaction_history::Model>> {
        let res = self
            .storage
            .get_operation_history(
                addr,
                OperationType::Delegate.into(),
                event,
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
        start_epoch: u64,
        end_epoch: u64,
        operation: u32,
    ) -> RpcResult<Vec<StakeAmount>> {
        let len = end_epoch - start_epoch;
        let mut ret = Vec::with_capacity(len as usize);

        for e in start_epoch..end_epoch {
            let res = self
                .storage
                .get_amount_by_epoch(e, operation)
                .await
                .map_err(ApiError::from)?;
            ret.push(res)
        }

        Ok(ret)
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

    async fn get_delegate_records(&self, addr: Address) -> RpcResult<RpcDelegateDeltas> {
        let ret = self
            .kvdb
            .get_delegator_status(addr.as_bytes())
            .await
            .map_err(ApiError::from)?
            .map(|r| DelegateDeltas::decode(&r).unwrap())
            .unwrap_or_default();
        Ok(ret.into())
    }

    async fn get_delegate_requirement(&self, staker: Address) -> RpcResult<DelegateRequirement> {
        let requirement_type_id = Stake::get_delegate_requirement_type_id(
            self.ckb_client.as_ref(),
            &METADATA_TYPE_ID.load(),
            &to_ckb_h160(&staker),
            &XUDT_OWNER.load(),
        )
        .await
        .map_err(ApiError::from)?;

        let delegate_requirement_cell = Delegate::get_requirement_cell(
            self.ckb_client.as_ref(),
            Delegate::requirement_type(&METADATA_TYPE_ID.load(), &requirement_type_id),
        )
        .await
        .map_err(ApiError::from)?;

        let delegate_requirement_cell_bytes =
            delegate_requirement_cell.output_data.unwrap().into_bytes();
        let delegate_cell_info =
            DelegateCellData::new_unchecked(delegate_requirement_cell_bytes).delegate_requirement();

        Ok(DelegateRequirement {
            threshold:          to_u128(&delegate_cell_info.threshold()) as u64,
            max_delegator_size: to_u32(&delegate_cell_info.max_delegator_size()),
            commission_rate:    to_u8(&delegate_cell_info.commission_rate()),
        })
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
