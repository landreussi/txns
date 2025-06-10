use std::fs::File;

use domain::account::Account;
use error::Result;

pub mod csv;
pub mod domain;
pub mod error;

fn main() -> Result<()> {
    let path = std::env::args()
        .last()
        // SAFETY: this unwrap is fine once if no argument is passed, the iterator will contain the
        // binary name.
        .unwrap();

    let file = File::open(path)?;
    let txns = csv::read(file)?;
    let accounts = Account::from_transactions(txns)?;

    csv::write(accounts, std::io::stdout())?;

    Ok(())
}
