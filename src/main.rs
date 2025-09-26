use color_eyre::eyre::OptionExt as _;
use csv::ReaderBuilder;
use csv::Trim;

use crate::account::ClientsAccounts;
use crate::engine::PaymentEngine;
use crate::transaction::Transaction;

mod account;
mod engine;
mod report;
mod transaction;

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
            );
        }
    }

    report::write_csv_to_stdout(clients_accounts.as_inner().values())?;

    Ok(())
}
