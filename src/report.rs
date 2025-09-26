use color_eyre::eyre::OptionExt;
use csv::Writer;
use rust_decimal::Decimal;
use serde::Serialize;
use toyments::account::ClientAccount;
use toyments::transaction::ClientId;

pub fn write_csv_to_stdout<'a, I>(clients_accounts: I) -> color_eyre::Result<()>
where
    I: IntoIterator<Item = &'a ClientAccount>,
{
    let mut writer = Writer::from_writer(std::io::stdout());
    for client_account in clients_accounts {
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
