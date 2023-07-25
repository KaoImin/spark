use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use ckb_jsonrpc_types::TransactionView;
use common::types::api::DelegateItem;
use jsonrpsee::core::{async_trait, RpcResult};
use rpc_client::ckb_client::ckb_rpc_client::CkbRpcClient;
use tx_builder::ckb::{
    delegate::DelegateTxBuilder, stake::StakeTxBuilder, stake_type_ids, withdraw::WithdrawTxBuilder,
};

use crate::error::ApiError;
use crate::jsonrpc::OperationRpcServer;
use common::traits::tx_builder::{IDelegateTxBuilder, IStakeTxBuilder, IWithdrawTxBuilder};
use common::types::tx_builder::{DelegateItem as TDelegateItem, StakeItem};
use common::types::{api::OperationType, H160, H256};
use common::utils::convert::to_ckb_h160;

pub struct OperationRpc {
    ckb_client: Arc<CkbRpcClient>,

    current_epoch: Arc<AtomicU64>,
}

impl OperationRpc {
    pub fn new(ckb_client: Arc<CkbRpcClient>, current_epoch: Arc<AtomicU64>) -> Self {
        Self {
            ckb_client,
            current_epoch,
        }
    }
}

#[async_trait]
impl OperationRpcServer for OperationRpc {
    async fn set_stake_rate(
        &self,
        _address: H256,
        _stake_rate: u64,
        _delegate_rate: u64,
    ) -> RpcResult<String> {
        unimplemented!()
    }

    async fn stake(&self, address: H160, amount: u64) -> RpcResult<TransactionView> {
        let current_epoch = self.current_epoch.load(Ordering::SeqCst);
        let stake_item = StakeItem {
            is_increase:        true,
            amount:             amount as u128,
            inauguration_epoch: current_epoch + 2,
        };

        let tx = StakeTxBuilder::new(
            self.ckb_client.as_ref(),
            stake_type_ids(),
            to_ckb_h160(&address),
            current_epoch,
            stake_item,
            None,
        )
        .build_tx()
        .await
        .map_err(ApiError::from)?;

        Ok(tx.into())
    }

    async fn unstake(&self, address: H160, amount: u64) -> RpcResult<TransactionView> {
        let current_epoch = self.current_epoch.load(Ordering::SeqCst);
        let stake_item = StakeItem {
            is_increase:        false,
            amount:             amount as u128,
            inauguration_epoch: current_epoch + 2,
        };

        let tx = StakeTxBuilder::new(
            self.ckb_client.as_ref(),
            stake_type_ids(),
            to_ckb_h160(&address),
            current_epoch,
            stake_item,
            None,
        )
        .build_tx()
        .await
        .map_err(ApiError::from)?;

        Ok(tx.into())
    }

    async fn delegate(
        &self,
        address: H160,
        delegate_items: Vec<DelegateItem>,
    ) -> RpcResult<TransactionView> {
        let current_epoch = self.current_epoch.load(Ordering::SeqCst);
        let infos = delegate_items
            .into_iter()
            .map(|i| TDelegateItem {
                staker:             i.staker,
                total_amount:       i.amount as u128,
                amount:             i.amount as u128,
                is_increase:        i.is_increase,
                inauguration_epoch: current_epoch + 2,
            })
            .collect::<Vec<_>>();

        let tx = DelegateTxBuilder::new(
            self.ckb_client.as_ref(),
            stake_type_ids(),
            to_ckb_h160(&address),
            current_epoch,
            infos,
        )
        .build_tx()
        .await
        .map_err(ApiError::from)?;

        Ok(tx.into())
    }

    async fn undelegate(
        &self,
        address: H160,
        delegate_items: Vec<DelegateItem>,
    ) -> RpcResult<TransactionView> {
        let current_epoch = self.current_epoch.load(Ordering::SeqCst);
        let infos = delegate_items
            .into_iter()
            .map(|i| TDelegateItem {
                staker:             i.staker,
                total_amount:       i.amount as u128,
                amount:             i.amount as u128,
                is_increase:        i.is_increase,
                inauguration_epoch: current_epoch + 2,
            })
            .collect::<Vec<_>>();

        let tx = DelegateTxBuilder::new(
            self.ckb_client.as_ref(),
            stake_type_ids(),
            to_ckb_h160(&address),
            current_epoch,
            infos,
        )
        .build_tx()
        .await
        .map_err(ApiError::from)?;

        Ok(tx.into())
    }

    async fn withdraw_stake(
        &self,
        address: H160,
        _withdraw_type: OperationType,
    ) -> RpcResult<TransactionView> {
        let current_epoch = self.current_epoch.load(Ordering::SeqCst);

        let tx = WithdrawTxBuilder::new(
            self.ckb_client.as_ref(),
            stake_type_ids(),
            to_ckb_h160(&address),
            current_epoch,
        )
        .build_tx()
        .await
        .map_err(ApiError::from)?;

        Ok(tx.into())
    }

    async fn withdraw_rewards(&self, _address: H160) -> RpcResult<TransactionView> {
        unimplemented!()
    }

    async fn send_transaction(&self, tx: TransactionView) -> RpcResult<ckb_types::H256> {
        let hash = self
            .ckb_client
            .send_transaction(&tx.inner, None)
            .await
            .map_err(ApiError::from)?;
        Ok(hash)
    }
}
