pub mod axon;
pub mod ckb;

use ckb_types::{bytes::Bytes, H256};
use common::types::tx_builder::NetworkType;
use std::sync::Arc;

pub fn init_static_variables(
    network_type: NetworkType,
    axon_token_type_args: Bytes,
    xudt_owner: H256,
    issuance_type_id: H256,
    metadata_type_id: H256,
    metadata_code_hash: H256,
    checkpoint_type_id: H256,
    stake_at_code_hash: H256,
    stake_smt_type_id: H256,
    delegate_at_code_hash: H256,
    delegate_smt_type_id: H256,
) {
    (*ckb::NETWORK_TYPE).swap(Arc::new(network_type));
    (*ckb::AXON_TOKEN_ARGS).swap(Arc::new(axon_token_type_args));
    (*ckb::XUDT_OWNER).swap(Arc::new(xudt_owner));
    (*ckb::ISSUANCE_TYPE_ID).swap(Arc::new(issuance_type_id));
    (*ckb::METADATA_TYPE_ID).swap(Arc::new(metadata_type_id));
    (*ckb::METADATA_CODE_HASH).swap(Arc::new(metadata_code_hash));
    (*ckb::CHECKPOINT_TYPE_ID).swap(Arc::new(checkpoint_type_id));
    (*ckb::STAKE_AT_CODE_HASH).swap(Arc::new(stake_at_code_hash));
    (*ckb::DELEGATE_AT_CODE_HASH).swap(Arc::new(delegate_at_code_hash));
    (*ckb::STAKE_SMT_CODE_HASH).swap(Arc::new(stake_smt_type_id));
    (*ckb::DELEGATE_SMT_CODE_HASH).swap(Arc::new(delegate_smt_type_id));
}
