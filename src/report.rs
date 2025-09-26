use csv::Writer;
use rust_decimal::Decimal;
use serde::Serialize;

use crate::account::ClientAccount;
use crate::transaction::ClientId;

pub fn write_csv_to_stdout<'a, I>(clients_accounts: I) -> color_eyre::Result<()>
where
    I: IntoIterator<Item = &'a ClientAccount>,
{
    let mut writer = Writer::from_writer(std::io::stdout());
    for client_account in clients_accounts.into_iter() {
        writer.serialize(ClientAccountReport::from(client_account))?;
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

impl From<&ClientAccount> for ClientAccountReport {
    fn from(client_account: &ClientAccount) -> Self {
        Self {
            client_id: client_account.client_id(),
            available: client_account.available(),
            held: client_account.held(),
            total: client_account.total(),
            locked: client_account.is_locked(),
        }
    }
}
