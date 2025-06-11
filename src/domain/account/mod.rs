use std::collections::HashMap;

use itertools::Itertools;
use rust_decimal::Decimal;
use serde::Serialize;

use super::{
    error::{Error, Result},
    transaction::{Transaction, TransactionKind},
};

#[derive(Debug, Serialize)]
pub struct Account {
    client: u16,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

impl Account {
    pub fn from_transactions(txns: impl IntoIterator<Item = Transaction>) -> Result<Vec<Self>> {
        let client_txns = txns.into_iter().into_group_map_by(|tx| tx.client);

        client_txns
            .into_iter()
            .map(|(client, txns)| Self::process_client_transactions(client, txns))
            .collect()
    }

    fn process_client_transactions(client: u16, mut txns: Vec<Transaction>) -> Result<Account> {
        let tx_amounts: HashMap<_, _> = txns
            .iter()
            .filter_map(|tx| match tx.kind {
                TransactionKind::Deposit { amount } => Some((tx.transaction_id, amount)),
                TransactionKind::Withdrawal { amount } => Some((tx.transaction_id, -amount)),
                _ => None,
            })
            .collect();

        let total = tx_amounts.values().sum();

        if total < Decimal::ZERO {
            return Err(Error::NoAvailableFundsToWithdraw { client });
        }

        txns.retain(|tx| {
            !matches!(
                tx.kind,
                TransactionKind::Deposit { .. } | TransactionKind::Withdrawal { .. }
            )
        });

        let (available, held, locked) = txns.iter().fold(
            (total, Decimal::ZERO, false),
            |(mut avail, mut held, mut locked), tx| {
                let amount = tx_amounts.get(&tx.transaction_id).unwrap_or(&Decimal::ZERO);

                match tx.kind {
                    TransactionKind::Dispute => {
                        avail -= amount;
                        held += amount;
                    }
                    TransactionKind::Resolve => {
                        avail += amount;
                        held -= amount;
                    }
                    TransactionKind::Chargeback => {
                        locked = true;
                    }
                    _ => unreachable!("processed txs was cleaned"),
                }

                (avail, held, locked)
            },
        );

        Ok(Account {
            client,
            total,
            available,
            held,
            locked,
        })
    }
}
