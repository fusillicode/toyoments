use csv::Writer;
use rust_decimal::Decimal;
use serde::Serialize;
use thiserror::Error;
use toyments::account::ClientAccount;
use toyments::transaction::ClientId;

#[derive(Debug, Error)]
pub enum CsvReportError {
    #[error("overflow in total calculation for client_account={client_account:?}")]
    TotalOverflow { client_account: ClientAccount },
    #[error("csv serialization error for client_account={client_account:?}, source_error={source:?}")]
    Csv {
        client_account: ClientAccount,
        #[source]
        source: csv::Error,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Write the supplied [`ClientAccount`]'s to stdout as CSV in ascending `client_id` order.
/// Returns a [`Vec`] of [`CsvReportError`] representing all the possible errors encountered during
/// reporting.
///
/// # Rationale
/// The sorting was introduced to match the expected output and to permit:
/// - Reproducible downstream processing
/// - Easier snapshot testing
///
/// The sorting was implemented at report time to keep
/// [`toyments::account::ClientsAccounts`] internal data structure an
/// [`std::collections::HashMap`] and permit fast inserts and updates (`O(1)` on average).
/// The cost of the ordering is a one‑shot `O(n log n)` when producing the final report.
/// This should be typically optimal for batch-style reporting at program end.
///
/// # Alternative
/// Switch to a [`std::collections::BTreeMap`] to have inherent ordering but
/// incur in an O(log n) cost for every mutation.
pub fn write_to_stdout<'a, I>(clients_accounts: I) -> Vec<CsvReportError>
where
    I: IntoIterator<Item = &'a ClientAccount>,
{
    let mut accounts: Vec<&ClientAccount> = clients_accounts.into_iter().collect();
    accounts.sort_unstable_by_key(|acc| acc.client_id());

    let mut writer = Writer::from_writer(std::io::stdout());
    let mut errors: Vec<CsvReportError> = Vec::new();

    for client_account in accounts {
        match ClientAccountReport::try_from(client_account) {
            Ok(report) => {
                if let Err(source) = writer.serialize(report) {
                    errors.push(CsvReportError::Csv {
                        client_account: *client_account,
                        source,
                    });
                }
            }
            Err(err) => errors.push(err),
        }
    }

    if let Err(io_err) = writer.flush() {
        errors.push(CsvReportError::Io(io_err));
    }

    errors
}

#[derive(Serialize)]
struct ClientAccountReport {
    client_id: ClientId,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,
}

impl TryFrom<&ClientAccount> for ClientAccountReport {
    type Error = CsvReportError;

    fn try_from(client_account: &ClientAccount) -> Result<Self, Self::Error> {
        Ok(Self {
            client_id: client_account.client_id(),
            available: client_account.available(),
            held: client_account.held(),
            total: client_account.total().ok_or(CsvReportError::TotalOverflow {
                client_account: *client_account,
            })?,
            locked: client_account.is_locked(),
        })
    }
}
