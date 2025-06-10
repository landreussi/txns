use std::collections::HashMap;

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
        let mut client_txns = HashMap::new();

        for tx in txns {
            client_txns
                .entry(tx.client)
                .and_modify(|txns: &mut Vec<_>| txns.push(tx.clone()))
                .or_insert(vec![tx]);
        }

        let mut accounts = Vec::with_capacity(client_txns.len());

        for (client, txns) in &client_txns {
            let mut tx_map = HashMap::new();

            for tx in txns {
                let amount = match tx.kind {
                    TransactionKind::Deposit { amount } => amount,
                    TransactionKind::Withdrawal { amount } => -amount,
                    _ => Decimal::ZERO,
                };

                tx_map.insert(tx.transaction_id, amount);
            }

            let total = tx_map.values().sum();
            let client = *client;

            if total < Decimal::ZERO {
                return Err(Error::NoAvailableFundsToWithdraw { client });
            }

            let mut available = total;
            let mut held = Decimal::ZERO;
            let mut locked = false;

            for tx in txns {
                if let Some(amount) = tx_map.get(&tx.transaction_id) {
                    match tx.kind {
                        TransactionKind::Dispute => {
                            available -= amount;
                            held += amount;
                        }
                        TransactionKind::Resolve => {
                            available += amount;
                            held -= amount;
                        }
                        TransactionKind::Chargeback => locked = true,
                        _ => {}
                    }
                }
            }

            accounts.push(Account {
                client,
                total,
                available,
                held,
                locked,
            });
        }

        Ok(accounts)
    }
}
