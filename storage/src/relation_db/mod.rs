use crate::error::StorageError;
use anyhow::Result;
use common::types::{
    api::{RewardHistory, StakeAmount},
    relation_db::{total_amount, transaction_history},
    smt::Address,
};
use migration::{Migrator, MigratorTrait};
pub use sea_orm::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, CursorTrait, Database, DbConn, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect,
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

    pub async fn get_id(&self) -> Result<u64> {
        let id = transaction_history::Entity::find()
            .order_by_desc(transaction_history::Column::Id)
            .one(&self.db)
            .await?
            .map(|tx| tx.id)
            .unwrap_or_default();
        Ok(id)
    }
}

/// Impl insert functions
impl RelationDB {
    pub async fn insert(&mut self, tx_record: transaction_history::ActiveModel) -> Result<()> {
        let tx_record = tx_record.insert(&self.db).await?;
        log::info!(
            "Transaction created with address: {}, timestamp: {}, tx_hash: {}",
            tx_record.address,
            tx_record.timestamp,
            tx_record.tx_hash
        );
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
        event: u32,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<transaction_history::Model>> {
        let mut cursor = transaction_history::Entity::find()
            .filter(transaction_history::Column::Address.eq(addr.to_string()))
            .filter(transaction_history::Column::Event.eq(event))
            .filter(transaction_history::Column::Operation.eq(operation))
            .cursor_by(transaction_history::Column::Id);
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
            amount: res
                .iter()
                .map(|r| r.amount as u128)
                .sum::<u128>()
                .to_string(),
        })
    }

    pub async fn get_reward_history(
        &self,
        addr: Address,
        page: u64,
        limit: u64,
    ) -> Result<Vec<RewardHistory>> {
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

    pub async fn get_address_state(&self, addr: Address) -> Result<total_amount::Model> {
        let res = total_amount::Entity::find()
            .filter(total_amount::Column::Address.eq(addr.to_string()))
            .all(&self.db)
            .await?;
        Ok(res[0].clone())
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
}
