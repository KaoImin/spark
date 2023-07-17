use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use ckb_sdk::{ScriptGroup, ScriptGroupType};
use ckb_types::{
    bytes::Bytes,
    core::{Capacity, TransactionBuilder, TransactionView},
    packed::{CellInput, CellOutput, WitnessArgs},
    prelude::{Entity, Pack},
};
use molecule::prelude::Builder;

use common::traits::{
    ckb_rpc_client::CkbRpc, smt::StakeSmtStorage, tx_builder::IStakeSmtTxBuilder,
};
use common::types::axon_types::basic::Byte32;
use common::types::axon_types::stake::{StakeArgs, StakeAtCellData, StakeSmtCellData};
use common::types::ckb_rpc_client::Cell;
use common::types::smt::{Root, Staker as SmtStaker, UserAmount};
use common::types::tx_builder::{
    Amount, Epoch, InStakeSmt, NonTopStakers, PrivateKey, StakeItem, StakeSmtTypeIds,
    Staker as TxStaker,
};
use common::utils::convert::new_u128;

use crate::ckb::define::{
    constants::{INAUGURATION, TOKEN_BYTES},
    error::CkbTxErr,
    types::StakeInfo,
};
use crate::ckb::helper::{
    token_cell_data, AlwaysSuccess, Checkpoint, Metadata, OmniEth, Secp256k1, Stake, Tx, Withdraw,
    Xudt,
};

pub struct StakeSmtTxBuilder<'a, C: CkbRpc, S: StakeSmtStorage + Send + Sync> {
    ckb:               &'a C,
    kicker:            PrivateKey,
    current_epoch:     Epoch,
    quorum:            u16,
    stake_cells:       Vec<Cell>,
    stake_smt_storage: S,
    type_ids:          StakeSmtTypeIds,
}

#[async_trait]
impl<'a, C: CkbRpc, S: StakeSmtStorage + Send + Sync> IStakeSmtTxBuilder<'a, C, S>
    for StakeSmtTxBuilder<'a, C, S>
{
    fn new(
        ckb: &'a C,
        kicker: PrivateKey,
        current_epoch: Epoch,
        type_ids: StakeSmtTypeIds,
        quorum: u16,
        stake_cells: Vec<Cell>,
        stake_smt_storage: S,
    ) -> Self {
        Self {
            ckb,
            kicker,
            current_epoch,
            quorum,
            stake_cells,
            stake_smt_storage,
            type_ids,
        }
    }

    async fn build_tx(&self) -> Result<(TransactionView, NonTopStakers)> {
        let stake_smt_type = Stake::smt_type(&self.type_ids.stake_smt_type_id);
        let stake_smt_cell = Stake::get_smt_cell(self.ckb, stake_smt_type.clone()).await?;

        let mut inputs = vec![
            // stake smt cell
            CellInput::new_builder()
                .previous_output(stake_smt_cell.out_point.clone().into())
                .build(),
        ];

        let (new_smt_root, cells, statistics, smt_witness) = self.collect().await?;

        let old_stake_smt_cell_bytes = stake_smt_cell.output_data.unwrap().into_bytes();
        let old_stake_smt_cell_data = StakeSmtCellData::new_unchecked(old_stake_smt_cell_bytes);
        let new_stake_smt_cell_data = old_stake_smt_cell_data
            .as_builder()
            .smt_root(Byte32::from_slice(new_smt_root.as_slice()).unwrap())
            .build()
            .as_bytes();

        let mut outputs = vec![
            // stake smt cell
            CellOutput::new_builder()
                .lock(AlwaysSuccess::lock())
                .type_(Some(stake_smt_type).pack())
                .build_exact_capacity(Capacity::bytes(new_stake_smt_cell_data.len())?)?,
        ];

        let mut outputs_data = vec![new_stake_smt_cell_data];
        let mut witnesses = vec![smt_witness.as_bytes()];

        // insert stake AT cells and withdraw AT cells to the transaction
        self.fill_tx(
            &statistics,
            &cells,
            &mut inputs,
            &mut outputs,
            &mut outputs_data,
            &mut witnesses,
        )
        .await?;

        witnesses.push(OmniEth::witness_placeholder().as_bytes()); // capacity provider lock

        let mut cell_deps = vec![
            OmniEth::lock_dep(),
            Secp256k1::lock_dep(),
            Xudt::type_dep(),
            AlwaysSuccess::lock_dep(),
            Stake::lock_dep(),
            Stake::smt_type_dep(),
            Checkpoint::cell_dep(self.ckb, &self.type_ids.checkpoint_type_id).await?,
            Metadata::cell_dep(self.ckb, &self.type_ids.metadata_type_id).await?,
        ];

        if !statistics.withdraw_amounts.is_empty() {
            cell_deps.push(Withdraw::lock_dep());
        }

        let tx = TransactionBuilder::default()
            .inputs(inputs)
            .outputs(outputs)
            .outputs_data(outputs_data.pack())
            .cell_deps(cell_deps)
            .witnesses(witnesses.pack())
            .build();

        let omni_eth = OmniEth::new(self.kicker.clone());
        let kicker_lock = OmniEth::lock(&omni_eth.address()?);

        let mut tx = Tx::new(self.ckb, tx);
        tx.balance(kicker_lock.clone()).await?;

        tx.sign(&omni_eth.signer()?, &ScriptGroup {
            script:         kicker_lock,
            group_type:     ScriptGroupType::Lock,
            input_indices:  vec![tx.inner_ref().witnesses().len() - 1],
            output_indices: vec![],
        })?;

        Ok((tx.inner(), statistics.non_top_stakers))
    }
}

struct Statistics {
    pub non_top_stakers:  HashMap<TxStaker, InStakeSmt>,
    pub withdraw_amounts: HashMap<TxStaker, Amount>,
}

impl<'a, C: CkbRpc, S: StakeSmtStorage + Send + Sync> StakeSmtTxBuilder<'a, C, S> {
    async fn fill_tx(
        &self,
        statistics: &Statistics,
        inputs_stake_cells: &HashMap<TxStaker, Cell>,
        inputs: &mut Vec<CellInput>,
        outputs: &mut Vec<CellOutput>,
        outputs_data: &mut Vec<Bytes>,
        witnesses: &mut Vec<Bytes>,
    ) -> Result<()> {
        let xudt = Xudt::type_(&self.type_ids.xudt_owner.pack());
        for (staker, stake_cell) in inputs_stake_cells.iter() {
            // inputs: stake AT cell
            inputs.push(
                CellInput::new_builder()
                    .previous_output(inputs_stake_cells[staker].out_point.clone().into())
                    .build(),
            );

            witnesses.push(Stake::witness(1).as_bytes());

            let (old_total_stake_amount, old_stake_data) = self.parse_stake_data(stake_cell);

            let withdraw_lock = Withdraw::lock(&self.type_ids.metadata_type_id, staker);

            let (new_total_stake_amount, withdraw_data) =
                if statistics.withdraw_amounts.contains_key(staker) {
                    let withdraw_amount =
                        statistics.withdraw_amounts.get(staker).unwrap().to_owned();

                    let old_withdraw_cell =
                        Withdraw::get_cell(self.ckb, withdraw_lock.clone(), xudt.clone())
                            .await?
                            .unwrap();

                    // inputs: withdraw AT cell
                    inputs.push(
                        CellInput::new_builder()
                            .previous_output(old_withdraw_cell.out_point.clone().into())
                            .build(),
                    );
                    witnesses.push(Withdraw::witness(true).as_bytes());

                    (
                        old_total_stake_amount - withdraw_amount,
                        Some(Withdraw::update_cell_data(
                            old_withdraw_cell,
                            self.current_epoch + INAUGURATION,
                            withdraw_amount,
                        )),
                    )
                } else {
                    (old_total_stake_amount, None)
                };

            let inner_stake_data = old_stake_data.lock();
            let new_stake_data = old_stake_data
                .as_builder()
                .lock(
                    inner_stake_data
                        .as_builder()
                        .delta(StakeItem::default().into())
                        .build(),
                )
                .build()
                .as_bytes();

            // outputs: stake AT cell
            outputs_data.push(token_cell_data(new_total_stake_amount, new_stake_data));
            outputs.push(
                CellOutput::new_builder()
                    .lock(Stake::lock(&self.type_ids.metadata_type_id, staker))
                    .type_(Some(xudt.clone()).pack())
                    .build_exact_capacity(Capacity::bytes(outputs_data.last().unwrap().len())?)?,
            );

            // outputs: withdraw AT cell
            if withdraw_data.is_some() {
                outputs.push(
                    CellOutput::new_builder()
                        .lock(withdraw_lock)
                        .type_(Some(xudt.clone()).pack())
                        .build_exact_capacity(Capacity::bytes(
                            withdraw_data.as_ref().unwrap().len(),
                        )?)?,
                );
                outputs_data.push(withdraw_data.unwrap());
            }
        }

        Ok(())
    }

    fn parse_stake_data(&self, cell: &Cell) -> (Amount, StakeAtCellData) {
        let mut cell_data_bytes = cell.output_data.clone().unwrap().into_bytes();
        let total_stake_amount = new_u128(&cell_data_bytes[..TOKEN_BYTES]);
        let stake_data = StakeAtCellData::new_unchecked(cell_data_bytes.split_off(TOKEN_BYTES));
        (total_stake_amount, stake_data)
    }

    async fn update_stake_smt(&self, new_smt: HashMap<SmtStaker, Amount>) -> Result<Root> {
        let new_smt_stakers = new_smt
            .iter()
            .map(|(k, v)| UserAmount {
                user:        k.to_owned(),
                amount:      v.to_owned(),
                is_increase: true,
            })
            .collect();

        self.stake_smt_storage
            .insert(self.current_epoch + INAUGURATION, new_smt_stakers)
            .await?;

        self.stake_smt_storage.get_top_root().await
    }

    async fn collect(&self) -> Result<(Root, HashMap<TxStaker, Cell>, Statistics, WitnessArgs)> {
        let old_smt = self
            .stake_smt_storage
            .get_sub_leaves(self.current_epoch + INAUGURATION)
            .await?;

        let xudt = Xudt::type_(&self.type_ids.xudt_owner.pack());

        let mut new_smt = old_smt.clone();
        let mut withdraw_amounts = HashMap::new(); // records all the stakers' withdraw amounts
        let mut inputs_stake_cells = HashMap::new();

        for cell in self.stake_cells.clone().into_iter() {
            let staker = TxStaker::from_slice(
                &StakeArgs::new_unchecked(cell.output.lock.args.as_bytes().to_owned().into())
                    .stake_addr()
                    .as_bytes(),
            )?;

            let (_, stake_data) = self.parse_stake_data(&cell);
            let stake_delta = Stake::item(&stake_data.lock().delta());

            if stake_delta.inauguration_epoch < self.current_epoch + INAUGURATION {
                continue;
            }

            inputs_stake_cells.insert(staker.clone(), cell);

            let smt_staker = SmtStaker::from(staker.0);
            if new_smt.contains_key(&smt_staker) {
                let origin_stake_amount = new_smt.get(&smt_staker).unwrap().to_owned();
                if stake_delta.is_increase {
                    new_smt.insert(smt_staker, origin_stake_amount + stake_delta.amount);
                } else if origin_stake_amount < stake_delta.amount {
                    withdraw_amounts.insert(staker, origin_stake_amount);
                } else {
                    new_smt.insert(smt_staker, origin_stake_amount - stake_delta.amount);
                    withdraw_amounts.insert(staker, stake_delta.amount);
                };
            } else {
                if !stake_delta.is_increase {
                    return Err(CkbTxErr::Increase(stake_delta.is_increase).into());
                }
                new_smt.insert(smt_staker, stake_delta.amount);
            }
        }

        let non_top_stakers = self.collect_non_top_stakers(&old_smt, &mut new_smt);

        for (staker, in_smt) in non_top_stakers.iter() {
            let smt_staker = SmtStaker::from(staker.0);
            if *in_smt {
                withdraw_amounts
                    .insert(staker.clone(), old_smt.get(&smt_staker).unwrap().to_owned());

                // It represents the case where the staker doesn't update its staking but is
                // removed from the smt since it's no longer the top stakers. In this case, the
                // staker's stake cell needs to be updated. So the cell should be put to the
                // inputs.
                if !inputs_stake_cells.contains_key(staker) {
                    let stake_cell = Stake::get_cell(
                        self.ckb,
                        Stake::lock(&self.type_ids.metadata_type_id, staker),
                        xudt.clone(),
                    )
                    .await?
                    .unwrap();

                    inputs_stake_cells.insert(staker.clone(), stake_cell);
                }
            } else {
                inputs_stake_cells.remove(staker);
            }
        }

        // get the old epoch proof for witness
        let old_epoch_proof = self
            .stake_smt_storage
            .generate_top_proof(vec![self.current_epoch + INAUGURATION])
            .await?;

        let new_root = self.update_stake_smt(new_smt.clone()).await?;

        // get the new epoch proof for witness
        let new_epoch_proof = self
            .stake_smt_storage
            .generate_top_proof(vec![self.current_epoch + INAUGURATION])
            .await?;

        let stake_smt_witness = Stake::smt_witness(
            0,
            old_smt
                .into_iter()
                .map(|(addr, amount)| StakeInfo {
                    addr: ckb_types::H160(addr.0),
                    amount,
                })
                .collect(),
            old_epoch_proof,
            new_epoch_proof,
        );

        Ok((
            new_root,
            inputs_stake_cells,
            Statistics {
                non_top_stakers,
                withdraw_amounts,
            },
            stake_smt_witness,
        ))
    }

    fn collect_non_top_stakers(
        &self,
        old_smt: &HashMap<SmtStaker, Amount>,
        new_smt: &mut HashMap<SmtStaker, Amount>,
    ) -> NonTopStakers {
        if new_smt.len() <= 3 * self.quorum as usize {
            return HashMap::default();
        }

        let mut all_stakes = new_smt
            .clone()
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect::<Vec<(SmtStaker, Amount)>>();
        all_stakes.sort_unstable_by_key(|v| v.1);

        let delete_count = all_stakes.len() - 3 * self.quorum as usize;
        let non_top_stakers = &all_stakes[..delete_count];

        non_top_stakers
            .iter()
            .map(|(staker, _)| {
                new_smt.remove(staker);
                (TxStaker::from(staker.0), old_smt.contains_key(staker))
            })
            .collect()
    }
}
