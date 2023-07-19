use std::{fs, path::Path};

use anyhow::Result;
use rocksdb::{prelude::*, ColumnFamilyDescriptor, DB};

const STAKE_COLUMN: &str = "c_stake";
const DELEGATE_COLUMN: &str = "c_delegate";

pub struct KVDB {
    db: DB,
}

impl KVDB {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        if !path.as_ref().is_dir() {
            fs::create_dir_all(&path).unwrap();
        }

        let categories = vec![STAKE_COLUMN, DELEGATE_COLUMN];
        let cf_descriptors = categories
            .into_iter()
            .map(|c| ColumnFamilyDescriptor::new(c, Options::default()))
            .collect::<Vec<_>>();

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        Self {
            db: DB::open_cf_descriptors(&opts, path, cf_descriptors).unwrap(),
        }
    }

    pub async fn insert_staker_status(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let stake_col = self.db.cf_handle(STAKE_COLUMN).unwrap();
        let ret = self.db.put_cf(stake_col, key, value)?;
        Ok(ret)
    }

    pub async fn get_staker_status(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let stake_col = self.db.cf_handle(STAKE_COLUMN).unwrap();
        let ret = self.db.get_cf(stake_col, key)?.map(|v| v.to_vec());
        Ok(ret)
    }

    pub async fn insert_delegator_status(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let delegate_col = self.db.cf_handle(DELEGATE_COLUMN).unwrap();
        let ret = self.db.put_cf(delegate_col, key, value)?;
        Ok(ret)
    }

    pub async fn get_delegator_status(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let delegate_col = self.db.cf_handle(DELEGATE_COLUMN).unwrap();
        let ret = self.db.get_cf(delegate_col, key)?.map(|v| v.to_vec());
        Ok(ret)
    }
}
