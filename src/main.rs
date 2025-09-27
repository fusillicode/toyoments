//! Streams transactions from a supplied CSV, mutates in‑memory client accounts (creating them if missing),
//! and emits a CSV report of the client accounts state.
//!
//! # Error Reporting Strategy
//!
//! * Errors are **reported immediately** to `stderr` when they occur in main (parse, business logic, or reporting
//!   failures) to ensure timely visibility.
//! * Each error is also **collected** in memory (`errors`) to:
//!   - Decide the **overall exit status** (`0` on success, `1` if any error).
//!   - Enable further processing like, classifying fatal vs non‑fatal errors, emits JSON representations, metrics, or
//!     dedicated summaries
//!
//! Avoids short‑circuiting on the first failure to preserve maximum successful work (best‑effort processing) at the
//! cost of possible inconsistencies.

use color_eyre::eyre::OptionExt as _;
use csv::ReaderBuilder;
use csv::Trim;
use toyments::account::ClientsAccounts;
use toyments::engine::PaymentEngine;
use toyments::engine::payment_engine::PaymentEngineError;
use toyments::transaction::Transaction;

use crate::csv_report::CsvReportError;

mod csv_report;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let tx_file_path = std::env::args().nth(1).ok_or_eyre("no transactions CSV supplied")?;
    let mut tx_file_reader = ReaderBuilder::new().trim(Trim::All).from_path(tx_file_path)?;

    let mut clients_accounts = ClientsAccounts::default();
    let mut payment_engine = PaymentEngine::default();

    let mut errors = vec![];
    for tx_res in tx_file_reader.deserialize::<Transaction>() {
        let tx = match tx_res {
            Ok(tx) => tx,
            Err(error) => {
                eprintln!("error deserializing transaction, error={error:?}");
                errors.push(ProcessingError::from(error));
                continue;
            }
        };

        let client_account = clients_accounts.get_or_create_new_account(tx.client_id());

        if let Err(error) = payment_engine.handle_transaction(client_account, tx) {
            eprintln!(
                "error handling transaction for client account, tx={tx:?}, client_account={client_account:?}, error={error:?}"
            );
            errors.push(ProcessingError::from(error));
        }
    }

    let report_errors = csv_report::write_to_stdout(clients_accounts.as_inner().values());
    for error in report_errors {
        eprintln!("error writing report: {error}");
        errors.push(ProcessingError::from(error));
    }

    if !errors.is_empty() {
        std::process::exit(1)
    }

    Ok(())
}

#[derive(thiserror::Error, Debug)]
enum ProcessingError {
    #[error(transparent)]
    Csv(#[from] csv::Error),
    #[error(transparent)]
    PaymentEngine(#[from] PaymentEngineError),
    #[error(transparent)]
    CsvReport(#[from] CsvReportError),
}
