use std::collections::BTreeMap;

use anyhow::Result;
use axon_types::delegate::DelegateInfoDelta;
use axon_types::stake::StakeInfoDelta;
use ckb_types::H160;
use serde::{Deserialize, Serialize};

use crate::utils::convert::to_h160;
use crate::utils::convert::to_u128;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct DelegateDeltas {
    pub inner: BTreeMap<H160, DelegateDelta>,
}

impl DelegateDeltas {
    pub fn encode(&self) -> Vec<u8> {
        let mut ret = vec![];
        ret.extend_from_slice(&(self.inner.len() as u32).to_le_bytes());

        for (_addr, delta) in &self.inner {
            ret.extend_from_slice(&delta.encode());
        }

        ret
    }

    pub fn decode(raw: &[u8]) -> Result<Self> {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&raw[0..4]);
        let len = u32::from_le_bytes(buf) as usize;
        let mut inner = BTreeMap::new();

        for i in 0..len {
            let offset = 4 + i * (20 + 17);
            let delta = DelegateDelta::decode(&raw[offset..offset + 37])?;
            inner.insert(delta.staker.clone(), delta);
        }

        Ok(DelegateDeltas { inner })
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct DelegateDelta {
    pub staker: H160,
    pub delta:  Delta,
}

impl From<&DelegateInfoDelta> for DelegateDelta {
    fn from(value: &DelegateInfoDelta) -> Self {
        Self {
            staker: to_h160(&value.staker()),
            delta:  Delta::from(value),
        }
    }
}

impl DelegateDelta {
    pub fn encode(&self) -> Vec<u8> {
        let mut ret = vec![];
        ret.extend_from_slice(&self.staker.0);
        ret.extend_from_slice(&self.delta.encode());

        ret
    }

    pub fn decode(raw: &[u8]) -> Result<Self> {
        let staker = H160::from_slice(&raw[0..20])?;
        let delta = Delta::decode(&raw[20..37])?;

        Ok(DelegateDelta { staker, delta })
    }

    pub fn sub(&self, other: &DelegateDelta) -> DelegateDelta {
        assert!(self.staker == other.staker);

        DelegateDelta {
            staker: self.staker.clone(),
            delta:  self.delta.sub(&other.delta),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Delta {
    pub is_increase: bool,
    pub amount:      u128,
}

impl Delta {
    pub fn amount(&self) -> i64 {
        if self.is_increase {
            self.amount as i64
        } else {
            -(self.amount as i64)
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = [0u8; 16];
        buf.copy_from_slice(&self.amount.to_le_bytes());

        let mut ret = vec![0u8; 17];
        ret[0] = if self.is_increase { 0 } else { 1 };
        ret[1..17].copy_from_slice(&buf);

        ret
    }

    pub fn decode(raw: &[u8]) -> Result<Self> {
        if raw.len() != 17 {
            return Err(anyhow::anyhow!("invalid delta length"));
        }

        let mut buf = [0u8; 16];
        buf.copy_from_slice(&raw[1..17]);

        Ok(Delta {
            is_increase: raw[0] == 0,
            amount:      u128::from_le_bytes(buf),
        })
    }

    pub fn sub(&self, other: &Delta) -> Delta {
        if self.is_increase == other.is_increase {
            return Delta {
                is_increase: self.is_increase,
                amount:      self.amount + other.amount,
            };
        }

        if self.is_increase && self.amount > other.amount {
            return Delta {
                is_increase: true,
                amount:      self.amount - other.amount,
            };
        } else if self.is_increase && self.amount < other.amount {
            return Delta {
                is_increase: false,
                amount:      other.amount - self.amount,
            };
        } else if !self.is_increase && self.amount > other.amount {
            return Delta {
                is_increase: false,
                amount:      self.amount - other.amount,
            };
        } else {
            return Delta {
                is_increase: true,
                amount:      other.amount - self.amount,
            };
        }
    }
}

impl From<&StakeInfoDelta> for Delta {
    fn from(delta: &StakeInfoDelta) -> Self {
        Self {
            is_increase: delta.is_increase() == 1u8.into(),
            amount:      to_u128(&delta.amount()),
        }
    }
}

impl From<&DelegateInfoDelta> for Delta {
    fn from(value: &DelegateInfoDelta) -> Self {
        Self {
            is_increase: value.is_increase() == 1u8.into(),
            amount:      to_u128(&value.amount()),
        }
    }
}
