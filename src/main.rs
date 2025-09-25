use std::collections::HashMap;

use color_eyre::eyre::OptionExt as _;
use csv::ReaderBuilder;
use csv::Trim;
use csv::Writer;
use rust_decimal::Decimal;
use serde::Serialize;

use crate::models::csv_models::ClientId;
use crate::models::csv_models::Transaction;

mod models;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let tx_file_path = std::env::args()
        .nth(1)
        .ok_or_eyre("no transactions CSV supplied")?;

    let mut tx_file_reader = ReaderBuilder::new()
        .trim(Trim::All)
        .from_path(tx_file_path)?;

    let mut clients_accounts: HashMap<ClientId, ClientAccount> = HashMap::new();

    for tx_res in tx_file_reader.deserialize::<Transaction>() {
        let Ok(tx) = tx_res else {
            eprintln!("error deserializing transaction, error={tx_res:?}");
            continue;
        };

        let client_id = tx.client_id();
        let client_account = clients_accounts
            .entry(client_id)
            .or_insert(ClientAccount::new(client_id));

        if client_account.locked {
            eprintln!(
                "client account locked, skip processing transaction, client_account={client_account:?}, tx={tx:?}"
            );
            continue;
        }
    }

    let mut writer = Writer::from_writer(std::io::stdout());
    for (_, client_account) in clients_accounts.iter() {
        writer.serialize(ClientAccountReport::from(client_account))?
    }

    Ok(())
}

#[derive(Debug, Copy, Clone)]
pub struct ClientAccount {
    client_id: ClientId,
    available: Decimal,
    held: Decimal,
    locked: bool,
}

impl ClientAccount {
    fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
            locked: false,
        }
    }

    fn total(&self) -> Decimal {
        self.available + self.held
    }
}

#[derive(Serialize)]
pub struct ClientAccountReport {
    client_id: ClientId,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

impl From<&ClientAccount> for ClientAccountReport {
    fn from(client_account: &ClientAccount) -> Self {
        Self {
            client_id: client_account.client_id,
            available: client_account.available,
            held: client_account.held,
            total: client_account.total(),
            locked: client_account.locked,
        }
    }
}
