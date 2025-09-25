use std::collections::HashMap;

use color_eyre::eyre::OptionExt as _;
use color_eyre::eyre::bail;
use csv::ReaderBuilder;
use csv::Trim;
use csv::Writer;
use rust_decimal::Decimal;
use serde::Serialize;

use crate::models::csv_models::ClientId;
use crate::models::csv_models::PositiveAmount;
use crate::models::csv_models::Transaction;
use crate::models::csv_models::TransactionId;

mod engine;
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
    let mut disputable_txs: HashMap<TransactionId, DisputableTransaction> = HashMap::new();

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

        match tx {
            Transaction::Deposit(deposit) => {
                client_account.available = client_account
                    .available
                    .checked_add(deposit.amount().as_inner())
                    .unwrap()
            }
            Transaction::Withdrawal(withdrawal) => {
                let withdrawal_amount = withdrawal.amount().as_inner();
                if client_account.available < withdrawal_amount {
                    bail!("not enough founds");
                }
                client_account.available = client_account
                    .available
                    .checked_sub(withdrawal.amount().as_inner())
                    .unwrap()
            }
            Transaction::Dispute(dispute) => {
                let disputed_tx_id = dispute.id();
                let Some(disputable_tx) = disputable_txs.get_mut(&disputed_tx_id) else {
                    continue;
                };

                if disputable_tx.id != disputed_tx_id {
                    bail!("mismatched ids")
                }

                if disputable_tx.is_disputed {
                    bail!("already disputed")
                }

                if disputable_tx.is_deposit()
                    && disputable_tx.amount.as_inner() < client_account.available
                {
                    bail!("cannot dispute, not enough available")
                }

                let signed_disputed_amount = disputable_tx.signed_amount();
                client_account.held = client_account
                    .held
                    .checked_sub(signed_disputed_amount)
                    .unwrap();
                if disputable_tx.is_deposit() {
                    client_account.available = client_account
                        .available
                        .checked_sub(signed_disputed_amount)
                        .unwrap();
                }

                disputable_tx.is_disputed = true;
            }
            Transaction::Resolve(resolve) => {
                let resolvable_tx_id = resolve.id();
                let Some(disputable_tx) = disputable_txs.get_mut(&resolvable_tx_id) else {
                    continue;
                };

                if disputable_tx.id != resolvable_tx_id {
                    bail!("mismatched ids")
                }

                if !disputable_tx.is_disputed {
                    bail!("tx not disputed")
                }

                let signed_disputed_amount = disputable_tx.signed_amount();
                client_account.held = client_account
                    .held
                    .checked_add(signed_disputed_amount)
                    .unwrap();
                client_account.available = client_account
                    .available
                    .checked_add(signed_disputed_amount)
                    .unwrap();

                disputable_tx.is_disputed = false;
            }
            Transaction::Chargeback(chargeback) => {
                let chargeback_tx_id = chargeback.id();
                let Some(disputable_tx) = disputable_txs.get_mut(&chargeback_tx_id) else {
                    continue;
                };

                if disputable_tx.id != chargeback_tx_id {
                    bail!("mismatched ids")
                }

                if !disputable_tx.is_disputed {
                    bail!("tx not disputed")
                }

                let signed_disputed_amount = disputable_tx.signed_amount();
                client_account.held = client_account
                    .held
                    .checked_add(signed_disputed_amount)
                    .unwrap();
                client_account.available = client_account
                    .available
                    .checked_add(signed_disputed_amount)
                    .unwrap();

                client_account.locked = true;
                disputable_tx.is_disputed = false;
            }
        };

        let Some(disputable_tx) = Option::<DisputableTransaction>::from(tx) else {
            continue;
        };
        disputable_txs.insert(disputable_tx.id, disputable_tx);
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

pub struct DisputableTransaction {
    id: TransactionId,
    amount: PositiveAmount,
    is_disputed: bool,
    kind: DisputableTransactionKind,
}

impl DisputableTransaction {
    pub fn is_deposit(&self) -> bool {
        self.kind.is_deposit()
    }

    pub fn signed_amount(&self) -> Decimal {
        match self.kind {
            DisputableTransactionKind::Deposit => self.amount.as_inner(),
            DisputableTransactionKind::Withdrawal => -self.amount.as_inner(),
        }
    }
}

impl From<Transaction> for Option<DisputableTransaction> {
    fn from(tx: Transaction) -> Self {
        let id = tx.id();
        match tx {
            Transaction::Deposit(deposit) => Some(DisputableTransaction {
                id,
                amount: deposit.amount(),
                is_disputed: false,
                kind: DisputableTransactionKind::Deposit,
            }),
            Transaction::Withdrawal(withdrawal) => Some(DisputableTransaction {
                id,
                amount: withdrawal.amount(),
                is_disputed: false,
                kind: DisputableTransactionKind::Withdrawal,
            }),
            Transaction::Dispute(_) | Transaction::Resolve(_) | Transaction::Chargeback(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum DisputableTransactionKind {
    Deposit,
    Withdrawal,
}

impl DisputableTransactionKind {
    fn is_deposit(&self) -> bool {
        match self {
            Self::Deposit => true,
            Self::Withdrawal => false,
        }
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
