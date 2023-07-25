mod error;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use ckb_jsonrpc_types::BlockView;
use ckb_jsonrpc_types::TransactionView;
use ckb_types::prelude::*;
use common::traits::smt::{DelegateSmtStorage, RewardSmtStorage, StakeSmtStorage};
use common::types::api::OperationType;
use common::types::axon_types::delegate::{DelegateArgs, DelegateAtCellData};
use common::types::axon_types::metadata::MetadataCellData;
use common::types::axon_types::stake::{StakeArgs, StakeAtCellData};
use common::types::delta::{DelegateDelta, DelegateDeltas, Delta};
use common::types::relation_db::transaction_history;
use common::types::smt::UserAmount;
use common::utils::convert::{to_eth_h160, to_h160, to_u64};
use rpc_client::ckb_client::ckb_rpc_client::CkbRpcClient;
use storage::{RelationDB, SmtManager, KVDB};
use tokio::time::sleep;
use tx_builder::ckb::{
    AXON_TOKEN_ARGS, DELEGATE_AT_CODE_HASH, DELEGATE_SMT_CODE_HASH, ISSUANCE_TYPE_ID,
    METADATA_CODE_HASH, METADATA_TYPE_ID, STAKE_AT_CODE_HASH, STAKE_SMT_CODE_HASH,
};

macro_rules! match_err {
    ($e: expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                log::error!("[sync] error: {:?}", e);
                continue;
            }
        }
    };
}

pub struct Synchronization {
    ckb_rpc_client: Arc<CkbRpcClient>,
    storage:        Arc<RelationDB>,
    kvdb:           Arc<KVDB>,
    stake_smt:      Arc<SmtManager>,
    delegate_smt:   Arc<SmtManager>,
    reward_smt:     Arc<SmtManager>,

    current_number: u64,
    current_epoch:  Arc<AtomicU64>,
}

impl Synchronization {
    pub async fn new(
        ckb_rpc_client: Arc<CkbRpcClient>,
        storage: Arc<RelationDB>,
        kvdb: Arc<KVDB>,
        stake_smt: Arc<SmtManager>,
        delegate_smt: Arc<SmtManager>,
        reward_smt: Arc<SmtManager>,
        current_number: u64,
        current_epoch: Arc<AtomicU64>,
    ) -> Self {
        let current_number = storage
            .get_latest_block_number()
            .await
            .unwrap()
            .unwrap_or(current_number);

        Self {
            ckb_rpc_client,
            storage,
            kvdb,
            stake_smt,
            delegate_smt,
            reward_smt,
            current_number,
            current_epoch,
        }
    }

    pub async fn run(mut self) {
        loop {
            let tip: u64 = match_err!(self.ckb_rpc_client.get_indexer_tip().await)
                .block_number
                .into();
            log::info!(
                "[sync] current number: {:?}, tip {:?}",
                self.current_number,
                tip
            );

            if tip - 24 > self.current_number {
                let current_number = self.current_number;
                let block = match_err!(
                    self.ckb_rpc_client
                        .get_block_by_number(current_number.into())
                        .await
                )
                .unwrap();

                let block_number: u64 = block.header.inner.number.into();

                log::info!("[sync] pull block: {:?}", block_number);

                self.parse_block(block).await.unwrap();
                self.current_number += 1;
            } else {
                sleep(Duration::from_secs(3)).await;
            }
        }
    }

    async fn parse_block(&self, block: BlockView) -> Result<()> {
        let block_number: u64 = block.header.inner.number.into();
        let timestamp: u64 = block.header.inner.timestamp.into();

        log::info!("[sync] parse block: {:?}", block_number);

        for tx in block.transactions.iter() {
            if let Some(epoch) = self.get_metadata_cell_epoch(tx) {
                log::info!("[sync] new epoch: {}", epoch);

                self.current_epoch.swap(epoch, Ordering::SeqCst);
                StakeSmtStorage::new_epoch(self.stake_smt.as_ref(), epoch).await?;
                DelegateSmtStorage::new_epoch(self.delegate_smt.as_ref(), epoch).await?;
                self.kvdb.insert_current_epoch(epoch).await?;
            } else if self.is_update_stake_smt_tx(tx) {
                continue;
            } else if self.is_delegate_smt_tx(tx) {
                continue;
            } else if let Some(i) = self.get_stake_tx_stake_at_cell_index(tx) {
                log::info!("[sync] handle stake tx: {} stake at index {}", tx.hash, i);

                self.handle_stake_tx(tx, i, timestamp, block_number).await?;
            } else if let Some(i) = self.get_delegate_tx_delegate_at_index(tx) {
                log::info!(
                    "[sync] handle delegate tx: {} delegate at index {}",
                    tx.hash,
                    i
                );

                self.handle_delegate_tx(tx, i, timestamp, block_number)
                    .await?;
            } else {
                continue;
            }
        }

        Ok(())
    }

    async fn handle_delegate_tx(
        &self,
        tx: &TransactionView,
        delegate_cell_index: usize,
        timestamp: u64,
        block_number: u64,
    ) -> Result<()> {
        let data = tx.inner.outputs_data[delegate_cell_index]
            .clone()
            .into_bytes()
            .split_off(16);
        let delegate_cell_data = DelegateAtCellData::new_unchecked(data);
        let cell_args = DelegateArgs::new_unchecked(
            tx.inner.outputs[delegate_cell_index]
                .lock
                .args
                .clone()
                .into_bytes(),
        );
        let delegator = to_h160(&cell_args.delegator_addr());

        log::info!("[sync] {} delegate", delegator);

        let raw = self.kvdb.get_delegator_status(&delegator.0).await?;
        let mut delegate_status = raw
            .map(|r| DelegateDeltas::decode(&r).unwrap())
            .unwrap_or_default();

        for item in delegate_cell_data.lock().delegator_infos().into_iter() {
            let staker = to_h160(&item.staker());
            log::info!("[sync] delegate to {}", staker);

            if !delegate_status.inner.contains_key(&staker) {
                delegate_status.inner.insert(staker.clone(), DelegateDelta {
                    staker: staker.clone(),
                    delta:  Default::default(),
                });
            }

            let original = delegate_status.inner.get_mut(&staker).unwrap();
            let new: DelegateDelta = (&item).into();
            let delta = new.sub(original);

            log::info!("[sync] delta is {:?}", delta);

            self.storage
                .insert_history(
                    transaction_history::Model {
                        id:        self.storage.get_id().await? + 1,
                        tx_hash:   tx.hash.clone().to_string(),
                        tx_block:  block_number as i64,
                        address:   delegator.to_string(),
                        amount:    delta.delta.amount(),
                        operation: OperationType::Delegate.into(),
                        event:     delta.delta.is_increase.into(),
                        epoch:     0,
                        status:    None,
                        timestamp: timestamp as i64,
                    }
                    .into(),
                )
                .await?;

            *original = new;
        }

        self.kvdb
            .insert_delegator_status(&delegator.0, &delegate_status.encode())
            .await?;

        Ok(())
    }

    async fn handle_stake_tx(
        &self,
        tx: &TransactionView,
        stake_cell_index: usize,
        timestamp: u64,
        block_number: u64,
    ) -> Result<()> {
        let data = tx.inner.outputs_data[stake_cell_index]
            .clone()
            .into_bytes()
            .split_off(16);
        let stake_cell_data = StakeAtCellData::new_unchecked(data);

        let cell_args = StakeArgs::new_unchecked(
            tx.inner.outputs[stake_cell_index]
                .lock
                .args
                .clone()
                .into_bytes(),
        );
        let staker = to_h160(&cell_args.stake_addr());

        let original = &self
            .kvdb
            .get_staker_status(&staker.0)
            .await?
            .map(|r| Delta::decode(&r).unwrap())
            .unwrap_or_default();
        let new: Delta = (&stake_cell_data.lock().delta()).into();
        let delta = new.sub(original);

        log::info!("[sync] delta is {:?}", delta);

        self.storage
            .insert_history(
                transaction_history::Model {
                    id:        self.storage.get_id().await? + 1,
                    tx_hash:   tx.hash.clone().to_string(),
                    tx_block:  block_number as i64,
                    address:   staker.to_string(),
                    amount:    delta.amount(),
                    operation: OperationType::Stake.into(),
                    event:     delta.is_increase.into(),
                    epoch:     0,
                    status:    None,
                    timestamp: timestamp as i64,
                }
                .into(),
            )
            .await?;
        self.kvdb
            .insert_staker_status(&staker.0, &new.encode())
            .await?;
        StakeSmtStorage::insert(
            self.stake_smt.as_ref(),
            self.current_epoch.load(Ordering::SeqCst),
            vec![UserAmount {
                user:        to_eth_h160(&staker),
                amount:      delta.amount() as u128,
                is_increase: delta.is_increase,
            }],
        )
        .await?;

        Ok(())
    }

    fn is_update_stake_smt_tx(&self, tx: &TransactionView) -> bool {
        if self.get_stake_tx_stake_at_cell_index(tx).is_some() {
            for c in tx.inner.outputs.iter() {
                if let Some(type_script) = c.type_.clone() {
                    if type_script.code_hash == **STAKE_SMT_CODE_HASH.load()
                        && type_script.args.as_bytes() == (**ISSUANCE_TYPE_ID.load()).as_ref()
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn is_delegate_smt_tx(&self, tx: &TransactionView) -> bool {
        if self.get_delegate_tx_delegate_at_index(tx).is_some() {
            for c in tx.inner.outputs.iter() {
                if let Some(type_script) = c.type_.clone() {
                    if type_script.code_hash == **DELEGATE_SMT_CODE_HASH.load()
                        && type_script.args.as_bytes() == (**ISSUANCE_TYPE_ID.load()).as_ref()
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn get_stake_tx_stake_at_cell_index(&self, tx: &TransactionView) -> Option<usize> {
        for (i, c) in tx.inner.outputs.iter().enumerate() {
            if let Some(type_script) = c.type_.clone() {
                if type_script.args.as_bytes() == **AXON_TOKEN_ARGS.load()
                    && c.lock.code_hash == **STAKE_AT_CODE_HASH.load()
                {
                    return Some(i);
                }
            }
        }

        None
    }

    fn get_delegate_tx_delegate_at_index(&self, tx: &TransactionView) -> Option<usize> {
        for (i, c) in tx.inner.outputs.iter().enumerate() {
            if let Some(type_script) = c.type_.clone() {
                if type_script.args.as_bytes() == **AXON_TOKEN_ARGS.load()
                    && c.lock.code_hash == **DELEGATE_AT_CODE_HASH.load()
                {
                    return Some(i);
                }
            }
        }

        None
    }

    fn get_metadata_cell_epoch(&self, tx: &TransactionView) -> Option<u64> {
        for (i, c) in tx.inner.outputs.iter().enumerate() {
            if let Some(type_script) = c.type_.clone() {
                if type_script.code_hash == **METADATA_CODE_HASH.load()
                    && type_script.args.as_bytes() == (*METADATA_TYPE_ID.load()).as_bytes()
                {
                    let data = MetadataCellData::new_unchecked(
                        tx.inner.outputs_data[i].clone().into_bytes(),
                    );
                    return Some(to_u64(&data.epoch()));
                }
            }
        }

        None
    }
}
