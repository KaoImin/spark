use crate::error::StorageError;
use anyhow::Result;
use common::{
    types::{
        api::{RewardHistory, StakeAmount},
        relation_db::{total_amount, transaction_history},
        smt::Address,
    },
    utils::codec::hex_encode,
};
use migration::{Migrator, MigratorTrait};
pub use sea_orm::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, CursorTrait, Database, DbConn, EntityTrait, IntoActiveModel,
    QueryFilter, QueryOrder, QuerySelect,
};

pub async fn establish_connection(database_url: &str) -> Result<DbConn> {
    let db = Database::connect(database_url).await?;
    Migrator::up(&db, None).await?;

    Ok(db)
}

pub struct RelationDB {
    pub db: DbConn,
}

impl RelationDB {
    pub async fn new(database_url: &str) -> Self {
        let db = establish_connection(database_url).await.unwrap();
        Self { db }
    }

    pub async fn get_id(&self) -> Result<i64> {
        let id = transaction_history::Entity::find()
            .order_by_desc(transaction_history::Column::Id)
            .one(&self.db)
            .await?
            .map(|tx| tx.id)
            .unwrap_or_default();
        Ok(id)
    }

    pub async fn get_status(&self, address: String) -> Result<Option<total_amount::Model>> {
        log::info!("get status with address: {}", address);
        let status = total_amount::Entity::find()
            .filter(total_amount::Column::Address.eq(address))
            .one(&self.db)
            .await?;
        Ok(status)
    }
}

/// Impl insert functions
impl RelationDB {
    pub async fn insert_history(&self, tx_record: transaction_history::ActiveModel) -> Result<()> {
        let status = self.get_status(tx_record.address.clone().unwrap()).await?;
        if status.is_none() {
            let mut total_amount = total_amount::ActiveModel {
                address:              tx_record.address.clone(),
                stake_amount:         Set(0),
                delegate_amount:      Set(0),
                withdrawable_amount:  Set(0),
                reward_lock_amount:   Set(0),
                reward_unlock_amount: Set(0),
            };
            match tx_record.operation.as_ref() {
                0 => {
                    total_amount.stake_amount = Set(*tx_record.amount.as_ref());
                }
                1 => {
                    total_amount.delegate_amount = Set(*tx_record.amount.as_ref());
                }
                2 => {
                    total_amount.reward_lock_amount = Set(*tx_record.amount.as_ref());
                }
                _ => {}
            }
            total_amount.insert(&self.db).await?;
        } else {
            let mut total_amount = status.unwrap().into_active_model();
            match tx_record.operation.as_ref() {
                0 => {
                    total_amount.stake_amount =
                        Set(total_amount.stake_amount.as_ref() + tx_record.amount.as_ref());
                }
                1 => {
                    total_amount.delegate_amount =
                        Set(total_amount.delegate_amount.as_ref() + tx_record.amount.as_ref());
                }
                2 => {
                    total_amount.reward_lock_amount =
                        Set(total_amount.reward_lock_amount.as_ref() + tx_record.amount.as_ref());
                }
                _ => {}
            }
            total_amount.update(&self.db).await?;
        }

        tx_record.clone().insert(&self.db).await?;

        log::info!(
            "Transaction created with address: {}, timestamp: {}, tx_hash: {}",
            tx_record.address.into_value().unwrap(),
            tx_record.timestamp.into_value().unwrap(),
            tx_record.tx_hash.into_value().unwrap()
        );
        Ok(())
    }

    pub async fn insert_total_amount(&self, staker: String) -> Result<()> {
        let status = total_amount::ActiveModel {
            address:              Set(staker),
            stake_amount:         Set(0),
            delegate_amount:      Set(0),
            withdrawable_amount:  Set(0),
            reward_lock_amount:   Set(0),
            reward_unlock_amount: Set(0),
        };

        self.inner_insert_total_amount(status).await
    }

    async fn inner_insert_total_amount(
        &self,
        total_amount: total_amount::ActiveModel,
    ) -> Result<()> {
        total_amount.insert(&self.db).await?;
        Ok(())
    }

    pub async fn add_stake_amount(&self, staker: String, amount: u128) -> Result<()> {
        let status = self.get_status(staker.clone()).await?;
        if status.is_none() {
            let s = total_amount::ActiveModel {
                address:              Set(staker),
                stake_amount:         Set(amount as i64),
                delegate_amount:      Set(0),
                withdrawable_amount:  Set(0),
                reward_lock_amount:   Set(0),
                reward_unlock_amount: Set(0),
            };
            self.inner_insert_total_amount(s).await?;
        }
        let mut total_amount = status.unwrap().into_active_model();
        total_amount.stake_amount = Set(total_amount.stake_amount.as_ref() + (amount as i64));
        total_amount.update(&self.db).await?;
        Ok(())
    }

    pub async fn redeem_stake_amount(&self, staker: String, amount: u128) -> Result<()> {
        let status = self.get_status(staker.clone()).await?;
        if status.is_none() {
            let s = total_amount::ActiveModel {
                address:              Set(staker),
                stake_amount:         Set(0),
                delegate_amount:      Set(0),
                withdrawable_amount:  Set(amount as i64),
                reward_lock_amount:   Set(0),
                reward_unlock_amount: Set(0),
            };
            self.inner_insert_total_amount(s).await?;
        }
        let mut total_amount = status.unwrap().into_active_model();
        total_amount.stake_amount = Set(total_amount.stake_amount.as_ref() - (amount as i64));
        total_amount.withdrawable_amount =
            Set(total_amount.withdrawable_amount.as_ref() + (amount as i64));
        total_amount.update(&self.db).await?;
        Ok(())
    }

    pub async fn add_delegate_amount(&self, staker: String, amount: u128) -> Result<()> {
        let status = self.get_status(staker.clone()).await?;
        if status.is_none() {
            let s = total_amount::ActiveModel {
                address:              Set(staker),
                stake_amount:         Set(0),
                delegate_amount:      Set(amount as i64),
                withdrawable_amount:  Set(0),
                reward_lock_amount:   Set(0),
                reward_unlock_amount: Set(0),
            };
            self.inner_insert_total_amount(s).await?;
        }
        let mut total_amount = status.unwrap().into_active_model();
        total_amount.delegate_amount = Set(total_amount.delegate_amount.as_ref() + (amount as i64));
        total_amount.update(&self.db).await?;
        Ok(())
    }

    pub async fn redeem_delegate_amount(&self, staker: String, amount: u128) -> Result<()> {
        let status = self.get_status(staker.clone()).await?;
        if status.is_none() {
            let s = total_amount::ActiveModel {
                address:              Set(staker),
                stake_amount:         Set(0),
                delegate_amount:      Set(0),
                withdrawable_amount:  Set(amount as i64),
                reward_lock_amount:   Set(0),
                reward_unlock_amount: Set(0),
            };
            self.inner_insert_total_amount(s).await?;
        }
        let mut total_amount = status.unwrap().into_active_model();
        total_amount.delegate_amount = Set(total_amount.delegate_amount.as_ref() - (amount as i64));
        total_amount.withdrawable_amount =
            Set(total_amount.withdrawable_amount.as_ref() + (amount as i64));
        total_amount.update(&self.db).await?;
        Ok(())
    }
}

/// Impl query functions
impl RelationDB {
    pub async fn get_records_by_address(
        &self,
        addr: Address,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<transaction_history::Model>> {
        let addr = hex_encode(addr);
        let mut cursor = transaction_history::Entity::find()
            .filter(transaction_history::Column::Address.eq(addr.to_string()))
            .cursor_by(transaction_history::Column::Id);
        cursor.after(offset).before(offset + limit);
        match cursor.all(&self.db).await {
            Ok(records) => Ok(records),
            Err(e) => Err(StorageError::SqlCursorError(e).into()),
        }
    }

    pub async fn get_operation_history(
        &self,
        addr: Address,
        operation: u32,
        event: Option<u32>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<transaction_history::Model>> {
        let addr = hex_encode(addr);
        let cursor = if let Some(evt) = event {
            transaction_history::Entity::find()
                .filter(transaction_history::Column::Address.eq(addr.to_string()))
                .filter(transaction_history::Column::Operation.eq(operation))
                .filter(transaction_history::Column::Event.eq(evt))
        } else {
            transaction_history::Entity::find()
                .filter(transaction_history::Column::Address.eq(addr.to_string()))
                .filter(transaction_history::Column::Operation.eq(operation))
        };

        let mut cursor = cursor.cursor_by(transaction_history::Column::Id);
        cursor.after(offset).before(offset + limit);

        match cursor.all(&self.db).await {
            Ok(records) => Ok(records),
            Err(e) => Err(StorageError::SqlCursorError(e).into()),
        }
    }

    pub async fn get_stake_amount_by_epoch(
        &self,
        operation: u32,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<transaction_history::Model>> {
        let mut cursor = transaction_history::Entity::find()
            .filter(transaction_history::Column::Operation.eq(operation))
            .cursor_by(transaction_history::Column::Id);
        cursor.after(offset).before(offset + limit);
        match cursor.all(&self.db).await {
            Ok(records) => Ok(records),
            Err(e) => Err(StorageError::SqlCursorError(e).into()),
        }
    }

    pub async fn get_amount_by_epoch(&self, epoch: u64, operation: u32) -> Result<StakeAmount> {
        let res = transaction_history::Entity::find()
            .filter(transaction_history::Column::Operation.eq(operation))
            .filter(transaction_history::Column::Epoch.eq(epoch))
            .all(&self.db)
            .await?;

        Ok(StakeAmount {
            epoch,
            amount: res.iter().map(|r| r.amount as u64).sum(),
        })
    }

    pub async fn get_reward_history(
        &self,
        addr: Address,
        page: u64,
        limit: u64,
    ) -> Result<Vec<RewardHistory>> {
        let addr = hex_encode(addr);
        let mut cursor = transaction_history::Entity::find()
            .filter(transaction_history::Column::Address.eq(addr.to_string()))
            .filter(transaction_history::Column::Operation.eq(1))
            .cursor_by(transaction_history::Column::Id);
        cursor.after(page).before(page + limit);
        // match cursor.all(&self.db).await {
        //     Ok(records) => {
        //         let mut res = Vec::new();
        //         for record in records {
        //             res.push(RewardHistory {
        //                 epoch: record.epoch,
        //                 amount: record.amount as u64,

        //             })
        //         }
        //         Ok(res)
        //     },
        //     Err(e) => Err(StorageError::SqlCursorError(e).into()),
        // }
        todo!()
    }

    pub async fn get_top_stake_address(&self, limit: u64) -> Result<Vec<total_amount::Model>> {
        let res = total_amount::Entity::find()
            .order_by_desc(total_amount::Column::StakeAmount)
            .limit(Some(limit))
            .all(&self.db)
            .await?;
        Ok(res)
    }

    pub async fn get_address_state(&self, addr: Address) -> Result<Option<total_amount::Model>> {
        let addr = hex_encode(addr);
        let res = total_amount::Entity::find()
            .filter(total_amount::Column::Address.eq(addr))
            .one(&self.db)
            .await?;
        Ok(res)
    }

    pub async fn get_latest_stake_transactions(
        &self,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<transaction_history::Model>> {
        let mut cursor = transaction_history::Entity::find()
            .order_by_desc(transaction_history::Column::Timestamp)
            .cursor_by(transaction_history::Column::Id);
        cursor.after(offset).before(offset + limit);
        match cursor.all(&self.db).await {
            Ok(records) => Ok(records),
            Err(e) => Err(StorageError::SqlCursorError(e).into()),
        }
    }

    pub async fn get_latest_stake_transaction_by_address(
        &self,
        address: Address,
    ) -> Result<Option<transaction_history::Model>> {
        let res = transaction_history::Entity::find()
            .filter(transaction_history::Column::Address.eq(address.to_string()))
            .order_by_desc(transaction_history::Column::Timestamp)
            .one(&self.db)
            .await?;
        Ok(res)
    }

    pub async fn get_latest_block_number(&self) -> Result<Option<u64>> {
        let res = transaction_history::Entity::find()
            .order_by_desc(transaction_history::Column::TxBlock)
            .one(&self.db)
            .await?;
        Ok(res.map(|r| r.tx_block as u64))
    }
}
