use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use ckb_jsonrpc_types::BlockView;
use rpc_client::ckb_client::ckb_rpc_client::CkbRpcClient;
use storage::RelationDB;
use tokio::time::sleep;

macro_rules! match_err {
    ($e: expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                println!("error: {:?}", e);
                continue;
            }
        }
    };
}

pub struct Synchronization {
    ckb_rpc_client: Arc<CkbRpcClient>,
    storage:        Arc<RelationDB>,
    current_number: u64,
}

impl Synchronization {
    pub fn new(
        ckb_rpc_client: Arc<CkbRpcClient>,
        storage: Arc<RelationDB>,
        current_number: u64,
    ) -> Self {
        Self {
            ckb_rpc_client,
            storage,
            current_number,
        }
    }

    pub async fn run(mut self) {
        loop {
            let tip: u64 = match_err!(self.ckb_rpc_client.get_indexer_tip().await)
                .block_number
                .into();

            if tip - 24 > self.current_number {
                let current_number = self.current_number;
                let block = match_err!(
                    self.ckb_rpc_client
                        .get_block_by_number(current_number.into())
                        .await
                )
                .unwrap();

                match_err!(self.parse_block(block).await);
                self.current_number += 1;
            } else {
                sleep(Duration::from_secs(3)).await;
            }
        }
    }

    async fn parse_block(&self, block: BlockView) -> Result<()> {
        let block_number: u64 = block.header.inner.number.into();
        let block_hash = block.header.hash;

        for tx in block.transactions.iter() {}

        Ok(())
    }
}
