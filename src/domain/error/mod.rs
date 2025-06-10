use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("withdrawn amount is bigger than deposited amount for client {client}")]
    NoAvailableFundsToWithdraw { client: u16 },
}

pub type Result<T> = std::result::Result<T, Error>;
