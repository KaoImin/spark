use crate::types::H160;
use ckb_types::H256;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DelegateRequirement {
    pub threshold:          u64,
    pub max_delegator_size: u32,
    pub commission_rate:    u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Pagination {
    pub page:  u64,
    pub limit: u64,
}

impl Pagination {
    pub fn offset(&self) -> u64 {
        self.page * self.limit
    }

    pub fn limit(&self) -> u64 {
        self.limit
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PaginationResult<T> {
    pub total: u64,
    pub data:  Vec<T>,
}

impl<T> PaginationResult<T> {
    pub fn new(data: Vec<T>) -> Self {
        PaginationResult {
            total: data.len() as u64,
            data,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ChainState {
    pub epoch:              u64,
    pub period:             u64,
    pub block_number:       u64,
    pub total_stake_amount: u64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum HistoryEvent {
    Add,
    Redeem,
    Withdraw,
}

impl From<HistoryEvent> for u32 {
    fn from(value: HistoryEvent) -> Self {
        match value {
            HistoryEvent::Add => 0,
            HistoryEvent::Redeem => 1,
            HistoryEvent::Withdraw => 2,
        }
    }
}

impl From<u32> for HistoryEvent {
    fn from(value: u32) -> Self {
        match value {
            0 => HistoryEvent::Add,
            1 => HistoryEvent::Redeem,
            2 => HistoryEvent::Withdraw,
            _ => panic!("Invalid value for HistoryEvent"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum OperationType {
    Stake,
    Delegate,
    Reward,
}

impl From<OperationType> for u32 {
    fn from(value: OperationType) -> Self {
        match value {
            OperationType::Stake => 0,
            OperationType::Delegate => 1,
            OperationType::Reward => 2,
        }
    }
}

impl From<u32> for OperationType {
    fn from(value: u32) -> Self {
        match value {
            0 => OperationType::Stake,
            1 => OperationType::Delegate,
            2 => OperationType::Reward,
            _ => panic!("Invalid value for OperationType"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum OperationStatus {
    Success,
    Pending,
    Failed,
}

impl From<u32> for OperationStatus {
    fn from(value: u32) -> Self {
        match value {
            0 => OperationStatus::Success,
            1 => OperationStatus::Pending,
            2 => OperationStatus::Failed,
            _ => panic!("Invalid value for OperationStatus"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum LockStatusType {
    Lock,
    Unlock,
}

impl From<u32> for LockStatusType {
    fn from(value: u32) -> Self {
        match value {
            0 => LockStatusType::Lock,
            1 => LockStatusType::Unlock,
            _ => panic!("Invalid value for LockStatusType"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StakeAmount {
    pub epoch:  u64,
    pub amount: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StakeRate {
    pub address:       H160,
    pub stake_rate:    f64,
    pub delegate_rate: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddressAmount {
    pub address: String,
    pub amount:  u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StakeState {
    pub total_amount:        u64,
    pub stake_amount:        u64,
    pub delegate_amount:     u64,
    pub withdrawable_amount: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StakeHistory {
    pub id:           String,
    pub amount:       u32,
    pub event:        HistoryEvent,
    pub status:       OperationStatus,
    pub transactions: Vec<HistoryTransactions>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryTransactions {
    pub hash:      H256,
    pub status:    OperationStatus,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RewardState {
    pub lock_amount:   u64,
    pub unlock_amount: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RewardHistory {
    pub epoch:  u64,
    pub amount: u64,
    pub locked: bool,
    pub from:   RewardFrom,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RewardFrom {
    pub reward_type: OperationType,
    pub address:     H160,
    pub amount:      u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StakeTransaction {
    pub timestamp: u64,
    pub hash:      H256,
    pub amount:    u64,
    pub status:    OperationStatus,
}
