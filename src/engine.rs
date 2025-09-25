use std::collections::HashMap;

use color_eyre::eyre::bail;

use crate::models::clients_accounts::ClientAccount;
use crate::models::csv_models::PositiveAmount;
use crate::models::csv_models::Transaction;
use crate::models::csv_models::TransactionId;

pub struct PaymentEngine {
    disputable_txs: HashMap<TransactionId, DisputableTransaction>,
}

impl PaymentEngine {
    pub fn new() -> Self {
        Self {
            disputable_txs: HashMap::new(),
        }
    }

    pub fn handle_transaction(
        &mut self,
        client_account: &mut ClientAccount,
        tx: Transaction,
    ) -> color_eyre::Result<()> {
        if client_account.client_id() != tx.client_id() {
            bail!("transaction not related to the account tx={tx:?}, account={client_account:?}")
        }

        if client_account.locked() {
            bail!(
                "client account locked, skip processing transaction, client_account={client_account:?}, tx={tx:?}"
            );
        }

        match tx {
            Transaction::Deposit(deposit) => client_account.deposit(deposit.amount())?,
            Transaction::Withdrawal(withdrawal) => client_account.withdraw(withdrawal.amount())?,
            Transaction::Dispute(dispute) => {
                let disputed_tx_id = dispute.id();
                let Some(disputable_tx) = self.disputable_txs.get_mut(&disputed_tx_id) else {
                    return Ok(());
                };

                if disputable_tx.id != disputed_tx_id {
                    bail!("mismatched ids")
                }

                if disputable_tx.is_disputed {
                    bail!("already disputed")
                }

                if disputable_tx.is_deposit() {
                    client_account.withdraw(disputable_tx.amount)?;
                    client_account.hold(disputable_tx.amount)?;
                } else {
                    client_account.hold(disputable_tx.amount)?;
                }

                disputable_tx.is_disputed = true;
            }
            Transaction::Resolve(resolve) => {
                let resolvable_tx_id = resolve.id();
                let Some(disputable_tx) = self.disputable_txs.get_mut(&resolvable_tx_id) else {
                    return Ok(());
                };

                if disputable_tx.id != resolvable_tx_id {
                    bail!("mismatched ids")
                }

                if !disputable_tx.is_disputed {
                    bail!("tx not disputed")
                }

                client_account.free(disputable_tx.amount)?;
                client_account.deposit(disputable_tx.amount)?;

                disputable_tx.is_disputed = false;
            }
            Transaction::Chargeback(chargeback) => {
                let chargeback_tx_id = chargeback.id();
                let Some(disputable_tx) = self.disputable_txs.get_mut(&chargeback_tx_id) else {
                    return Ok(());
                };

                if disputable_tx.id != chargeback_tx_id {
                    bail!("mismatched ids")
                }

                if !disputable_tx.is_disputed {
                    bail!("tx not disputed")
                }

                if disputable_tx.is_deposit() {
                    client_account.withdraw(disputable_tx.amount)?;
                    client_account.hold(disputable_tx.amount)?;
                } else {
                    client_account.hold(disputable_tx.amount)?;
                }

                if disputable_tx.is_deposit() {
                    client_account.free(disputable_tx.amount)?;
                } else {
                    client_account.deposit(disputable_tx.amount)?;
                    client_account.free(disputable_tx.amount)?;
                }

                client_account.lock();
                disputable_tx.is_disputed = false;
            }
        };

        let Some(disputable_tx) = Option::<DisputableTransaction>::from(tx) else {
            return Ok(());
        };
        self.disputable_txs.insert(disputable_tx.id, disputable_tx);

        Ok(())
    }
}

#[derive(Debug)]
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
