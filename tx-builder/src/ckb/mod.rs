pub mod checkpoint;
mod define;
pub mod delegate;
pub mod delegate_smt;
pub mod faucet;
pub mod helper;
pub mod init;
pub mod metadata;
pub mod mint;
pub mod reward;
pub mod stake;
pub mod stake_smt;
mod tests;
pub mod withdraw;

use arc_swap::ArcSwap;
use bytes::Bytes;
use ckb_types::H256;
use common::types::tx_builder::{NetworkType, StakeTypeIds};

lazy_static::lazy_static! {
    pub static ref NETWORK_TYPE: ArcSwap<NetworkType> = ArcSwap::from_pointee(NetworkType::Testnet);
    pub static ref AXON_TOKEN_ARGS: ArcSwap<Bytes> = ArcSwap::from_pointee(Bytes::default());
    pub static ref XUDT_OWNER: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    pub static ref ISSUANCE_TYPE_ID: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    pub static ref METADATA_TYPE_ID: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    pub static ref METADATA_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    pub static ref CHECKPOINT_TYPE_ID: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    pub static ref STAKE_AT_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    pub static ref DELEGATE_AT_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    pub static ref STAKE_SMT_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
    pub static ref DELEGATE_SMT_CODE_HASH: ArcSwap<H256> = ArcSwap::from_pointee(H256::default());
}

pub fn stake_type_ids() -> StakeTypeIds {
    StakeTypeIds {
        metadata_type_id:   (*METADATA_TYPE_ID.load_full()).clone(),
        checkpoint_type_id: (*CHECKPOINT_TYPE_ID.load_full()).clone(),
        xudt_owner:         (*XUDT_OWNER.load_full()).clone(),
    }
}
