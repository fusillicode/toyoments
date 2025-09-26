use color_eyre::eyre::OptionExt;
use csv::Writer;
use rust_decimal::Decimal;
use serde::Serialize;
use toyments::account::ClientAccount;
use toyments::transaction::ClientId;

/// Write the supplied `ClientAccount`s to stdout as CSV in ascending `client_id` order.
///
/// # Rationale
/// Deterministic ordering (sorted by `client_id`) yields reproducible output,
/// simpler diffs, and stable snapshot tests while retaining a `HashMap` for
/// amortized O(1) updates.
///
/// # Approach
/// Collect references, perform a one‑shot O(n log n) sort at report time, then
/// serialize to CSV.
///
/// # Alternative
/// Using a `BTreeMap` would provide inherent ordering but impose O(log n) on every
/// mutation even if no report is emitted.
///
/// # Errors
/// Returns an error if:
/// - Computing `total` overflows (from [`ClientAccountReport::try_from`]).
/// - Serializing a row fails ([`csv::Error`]).
/// - Flushing stdout fails (I/O error).
pub fn write_to_stdout<'a, I>(clients_accounts: I) -> color_eyre::Result<()>
where
    I: IntoIterator<Item = &'a ClientAccount>,
{
    let mut accounts: Vec<&ClientAccount> = clients_accounts.into_iter().collect();
    accounts.sort_unstable_by_key(|acc| acc.client_id());

    let mut writer = Writer::from_writer(std::io::stdout());
    for client_account in accounts {
        writer.serialize(ClientAccountReport::try_from(client_account)?)?;
    }
    writer.flush()?;

    Ok(())
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
    type Error = color_eyre::Report;

    fn try_from(client_account: &ClientAccount) -> Result<Self, Self::Error> {
        Ok(Self {
            client_id: client_account.client_id(),
            available: client_account.available(),
            held: client_account.held(),
            total: client_account.total().ok_or_eyre(format!(
                "overflow in total calculation for client_account={client_account:?}]"
            ))?,
            locked: client_account.is_locked(),
        })
    }
}
