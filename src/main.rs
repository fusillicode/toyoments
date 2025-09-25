use color_eyre::eyre::OptionExt as _;
use csv::ReaderBuilder;
use csv::Trim;
use csv::Writer;
use rust_decimal::Decimal;
use serde::Serialize;

use crate::models::clients_accounts::ClientAccount;
use crate::models::clients_accounts::ClientsAccounts;
use crate::models::csv_models::ClientId;
use crate::models::csv_models::Transaction;
use crate::payment_engine::PaymentEngine;

mod models;
mod payment_engine;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let tx_file_path = std::env::args().nth(1).ok_or_eyre("no transactions CSV supplied")?;

    let mut tx_file_reader = ReaderBuilder::new().trim(Trim::All).from_path(tx_file_path)?;

    let mut clients_accounts = ClientsAccounts::new();
    let mut payment_engine = PaymentEngine::new();

    for tx_res in tx_file_reader.deserialize::<Transaction>() {
        let Ok(tx) = tx_res else {
            eprintln!("error deserializing transaction, error={tx_res:?}");
            continue;
        };

        let client_account = clients_accounts.get_or_create_new_account(tx.client_id());

        if let Err(error) = payment_engine.handle_transaction(client_account, tx) {
            eprintln!(
                "error handling transaction for client account, tx={tx:?}, client_account={client_account:?}, error={error:?}"
            )
        }
    }

    let mut writer = Writer::from_writer(std::io::stdout());
    for (_, client_account) in clients_accounts.as_inner().iter() {
        writer.serialize(ClientAccountReport::from(client_account))?
    }

    Ok(())
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
            client_id: client_account.client_id(),
            available: client_account.available(),
            held: client_account.held(),
            total: client_account.total(),
            locked: client_account.locked(),
        }
    }
}
