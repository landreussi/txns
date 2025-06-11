use std::collections::{HashMap, HashSet};

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
            .map(|(client, txns)| {
                Self::process_client_transactions(client, txns.into_iter().collect())
            })
            .collect()
    }

    fn process_client_transactions(client: u16, txns: HashSet<Transaction>) -> Result<Account> {
        let tx_amounts: HashMap<_, _> = txns
            .iter()
            .filter_map(|tx| match tx.kind {
                TransactionKind::Deposit { amount } => Some((tx.transaction_id, amount)),
                TransactionKind::Withdrawal { amount } => Some((tx.transaction_id, -amount)),
                _ => None,
            })
            .collect();

        let mut total = tx_amounts.values().sum();

        if total < Decimal::ZERO {
            return Err(Error::NoAvailableFundsToWithdraw { client });
        }

        let (available, held, locked) = txns
            .iter()
            .filter(|tx| {
                !matches!(
                    tx.kind,
                    TransactionKind::Deposit { .. } | TransactionKind::Withdrawal { .. }
                )
            })
            .fold(
                (total, Decimal::ZERO, false),
                |(mut avail, mut held, mut locked), tx| {
                    let amount = tx_amounts.get(&tx.transaction_id).unwrap_or(&Decimal::ZERO);

                    match tx.kind {
                        TransactionKind::Dispute => {
                            // Disputing a withdrawal is a tricky question, but I think it should
                            // add value only in held field, leaving all other fields untouched.
                            if amount < &Decimal::ZERO {
                                avail -= amount;
                                held -= amount;
                                total -= amount;
                            } else {
                                avail -= amount;
                                held += amount;
                            }
                        }
                        TransactionKind::Resolve => {
                            if amount < &Decimal::ZERO {
                                held += amount;
                            } else {
                                avail += amount;
                                held -= amount;
                            }
                        }
                        TransactionKind::Chargeback => {
                            locked = true;
                        }
                        _ => unreachable!("processed txs was filtered above"),
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

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    #[test]
    fn single_deposit() {
        let transactions = vec![Transaction {
            client: 1,
            transaction_id: 1,
            kind: TransactionKind::Deposit {
                amount: dec!(100.0),
            },
        }];

        let accounts = Account::from_transactions(transactions).unwrap();
        assert_eq!(accounts.len(), 1);

        let account = &accounts[0];
        assert_eq!(account.client, 1);
        assert_eq!(account.total, dec!(100.0));
        assert_eq!(account.available, dec!(100.0));
        assert_eq!(account.held, dec!(0.0));
        assert!(!account.locked);
    }

    #[test]
    fn deposit_and_withdrawal() {
        let transactions = vec![
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Deposit {
                    amount: dec!(100.0),
                },
            },
            Transaction {
                client: 1,
                transaction_id: 2,
                kind: TransactionKind::Withdrawal { amount: dec!(30.0) },
            },
        ];

        let accounts = Account::from_transactions(transactions).unwrap();
        let account = &accounts[0];

        assert_eq!(account.total, dec!(70.0));
        assert_eq!(account.available, dec!(70.0));
        assert_eq!(account.held, dec!(0.0));
        assert!(!account.locked);
    }

    #[test]
    fn withdrawal_exceeds_balance() {
        let transactions = vec![
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Deposit { amount: dec!(50.0) },
            },
            Transaction {
                client: 1,
                transaction_id: 2,
                kind: TransactionKind::Withdrawal {
                    amount: dec!(100.0),
                },
            },
        ];

        let result = Account::from_transactions(transactions);
        let error = result.unwrap_err();

        // Used this to skip deriving Eq to Error.
        assert!(matches!(
            error,
            Error::NoAvailableFundsToWithdraw { client: 1 }
        ));
    }

    #[test]
    fn dispute_transaction() {
        let transactions = vec![
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Deposit {
                    amount: dec!(100.0),
                },
            },
            Transaction {
                client: 1,
                transaction_id: 2,
                kind: TransactionKind::Deposit { amount: dec!(50.0) },
            },
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Dispute,
            },
        ];

        let accounts = Account::from_transactions(transactions).unwrap();
        let account = &accounts[0];

        assert_eq!(account.total, dec!(150.0));
        assert_eq!(account.available, dec!(50.0));
        assert_eq!(account.held, dec!(100.0));
        assert!(!account.locked);
    }

    #[test]
    fn dispute_and_resolve() {
        let transactions = vec![
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Deposit {
                    amount: dec!(100.0),
                },
            },
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Dispute,
            },
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Resolve,
            },
        ];

        let accounts = Account::from_transactions(transactions).unwrap();
        let account = &accounts[0];

        assert_eq!(account.total, dec!(100.0));
        assert_eq!(account.available, dec!(100.0));
        assert_eq!(account.held, dec!(0.0));
        assert!(!account.locked);
    }

    #[test]
    fn dispute_and_chargeback() {
        let transactions = vec![
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Deposit {
                    amount: dec!(100.0),
                },
            },
            Transaction {
                client: 1,
                transaction_id: 2,
                kind: TransactionKind::Deposit { amount: dec!(50.0) },
            },
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Dispute,
            },
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Chargeback,
            },
        ];

        let accounts = Account::from_transactions(transactions).unwrap();
        let account = &accounts[0];

        assert_eq!(account.total, dec!(150.0));
        assert_eq!(account.available, dec!(50.0));
        assert_eq!(account.held, dec!(100.0));
        assert!(account.locked);
    }

    #[test]
    fn dispute_withdrawal() {
        let transactions = vec![
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Deposit {
                    amount: dec!(100.0),
                },
            },
            Transaction {
                client: 1,
                transaction_id: 2,
                kind: TransactionKind::Withdrawal { amount: dec!(30.0) },
            },
            Transaction {
                client: 1,
                transaction_id: 2,
                kind: TransactionKind::Dispute,
            },
        ];

        let accounts = Account::from_transactions(transactions).unwrap();
        let account = &accounts[0];

        assert_eq!(account.total, dec!(100.0));
        assert_eq!(account.available, dec!(100.0));
        assert_eq!(account.held, dec!(30.0));
        assert!(!account.locked);
    }

    #[test]
    fn dispute_and_resolve_withdrawal() {
        let transactions = vec![
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Deposit {
                    amount: dec!(100.0),
                },
            },
            Transaction {
                client: 1,
                transaction_id: 2,
                kind: TransactionKind::Withdrawal { amount: dec!(30.0) },
            },
            Transaction {
                client: 1,
                transaction_id: 2,
                kind: TransactionKind::Dispute,
            },
            Transaction {
                client: 1,
                transaction_id: 2,
                kind: TransactionKind::Resolve,
            },
        ];

        let accounts = Account::from_transactions(transactions).unwrap();
        let account = &accounts[0];

        assert_eq!(account.total, dec!(100.0));
        assert_eq!(account.available, dec!(100.0));
        assert_eq!(account.held, dec!(0.0));
        assert!(!account.locked);
    }

    #[test]
    fn dispute_nonexistent_transaction() {
        let transactions = vec![
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Deposit {
                    amount: dec!(100.0),
                },
            },
            Transaction {
                client: 1,
                transaction_id: 999,
                kind: TransactionKind::Dispute,
            },
        ];

        let accounts = Account::from_transactions(transactions).unwrap();
        let account = &accounts[0];

        assert_eq!(account.total, dec!(100.0));
        assert_eq!(account.available, dec!(100.0));
        assert_eq!(account.held, dec!(0.0));
        assert!(!account.locked);
    }

    #[test]
    fn multiple_clients() {
        let transactions = vec![
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Deposit {
                    amount: dec!(100.0),
                },
            },
            Transaction {
                client: 2,
                transaction_id: 2,
                kind: TransactionKind::Deposit {
                    amount: dec!(200.0),
                },
            },
            Transaction {
                client: 1,
                transaction_id: 3,
                kind: TransactionKind::Withdrawal { amount: dec!(20.0) },
            },
            Transaction {
                client: 2,
                transaction_id: 2,
                kind: TransactionKind::Dispute,
            },
        ];

        let mut accounts = Account::from_transactions(transactions).unwrap();
        accounts.sort_by_key(|a| a.client);

        assert_eq!(accounts.len(), 2);

        // Client 1
        assert_eq!(accounts[0].client, 1);
        assert_eq!(accounts[0].total, dec!(80.0));
        assert_eq!(accounts[0].available, dec!(80.0));
        assert_eq!(accounts[0].held, dec!(0.0));
        assert!(!accounts[0].locked);

        // Client 2
        assert_eq!(accounts[1].client, 2);
        assert_eq!(accounts[1].total, dec!(200.0));
        assert_eq!(accounts[1].available, dec!(0.0));
        assert_eq!(accounts[1].held, dec!(200.0));
        assert!(!accounts[1].locked);
    }

    #[test]
    fn only_disputes_and_resolves() {
        let transactions = vec![
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Dispute,
            },
            Transaction {
                client: 1,
                transaction_id: 2,
                kind: TransactionKind::Resolve,
            },
            Transaction {
                client: 1,
                transaction_id: 3,
                kind: TransactionKind::Chargeback,
            },
        ];

        let accounts = Account::from_transactions(transactions).unwrap();
        let account = &accounts[0];

        assert_eq!(account.total, dec!(0.0));
        assert_eq!(account.available, dec!(0.0));
        assert_eq!(account.held, dec!(0.0));
        assert!(account.locked);
    }

    #[test]
    fn complex_scenario() {
        let transactions = vec![
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Deposit {
                    amount: dec!(1000.0),
                },
            },
            Transaction {
                client: 1,
                transaction_id: 2,
                kind: TransactionKind::Deposit {
                    amount: dec!(500.0),
                },
            },
            Transaction {
                client: 1,
                transaction_id: 3,
                kind: TransactionKind::Withdrawal {
                    amount: dec!(200.0),
                },
            },
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Dispute,
            },
            Transaction {
                client: 1,
                transaction_id: 4,
                kind: TransactionKind::Deposit {
                    amount: dec!(100.0),
                },
            },
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Resolve,
            },
            Transaction {
                client: 1,
                transaction_id: 3,
                kind: TransactionKind::Dispute,
            },
        ];

        let accounts = Account::from_transactions(transactions).unwrap();
        let account = &accounts[0];

        assert_eq!(account.total, dec!(1600.0));
        assert_eq!(account.available, dec!(1600.0));
        assert_eq!(account.held, dec!(200.0));
        assert!(!account.locked);
    }

    #[test]
    fn precision_handling() {
        let transactions = vec![
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Deposit {
                    amount: dec!(100.1234),
                },
            },
            Transaction {
                client: 1,
                transaction_id: 2,
                kind: TransactionKind::Withdrawal {
                    amount: dec!(50.5678),
                },
            },
        ];

        let accounts = Account::from_transactions(transactions).unwrap();
        let account = &accounts[0];

        assert_eq!(account.total, dec!(49.5556));
        assert_eq!(account.available, dec!(49.5556));
    }

    #[test]
    fn multiple_disputes_same_transaction() {
        let transactions = vec![
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Deposit {
                    amount: dec!(100.0),
                },
            },
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Dispute,
            },
            Transaction {
                client: 1,
                transaction_id: 1,
                kind: TransactionKind::Dispute,
            },
        ];

        let accounts = Account::from_transactions(transactions).unwrap();
        let account = &accounts[0];

        assert_eq!(account.total, dec!(100.0));
        assert_eq!(account.available, dec!(0.0));
        assert_eq!(account.held, dec!(100.0));
        assert!(!account.locked);
    }
}
