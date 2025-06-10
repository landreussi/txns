use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Deserialize, Hash, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum TransactionKind {
    Deposit { amount: Decimal },
    Withdrawal { amount: Decimal },
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Deserialize, Hash, PartialEq, Eq, Clone)]
pub struct Transaction {
    #[serde(rename = "tx")]
    pub transaction_id: u64,
    pub client: u16,
    #[serde(flatten)]
    pub kind: TransactionKind,
}
