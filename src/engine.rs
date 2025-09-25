use std::collections::HashMap;

use crate::models::clients_accounts::ClientAccount;
use crate::models::clients_accounts::ClientAccountError;
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
            return Err(PaymentEngineError::UnrelatedTransaction {
                client_account: *client_account,
                tx,
            })?;
        }

        if client_account.locked() {
            return Err(PaymentEngineError::ClientAccountLocked {
                client_account: *client_account,
                tx,
            })?;
        }

        match tx {
            Transaction::Deposit(deposit) => client_account.deposit(deposit.amount())?,
            Transaction::Withdrawal(withdrawal) => client_account.withdraw(withdrawal.amount())?,
            Transaction::Dispute(dispute) => {
                let disputed_tx_id = dispute.id();
                let disputable_tx = self.get_disputable_transaction(disputed_tx_id)?;

                if disputable_tx.is_disputed {
                    return Err(PaymentEngineError::TransactionAlreadyDisputed {
                        client_account: *client_account,
                        tx,
                    })?;
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
                let disputable_tx = self.get_disputable_transaction(resolvable_tx_id)?;

                if !disputable_tx.is_disputed {
                    return Err(PaymentEngineError::TransactionNotDisputed {
                        client_account: *client_account,
                        tx,
                    })?;
                }

                client_account.unhold(disputable_tx.amount)?;
                client_account.deposit(disputable_tx.amount)?;

                disputable_tx.is_disputed = false;
            }
            Transaction::Chargeback(chargeback) => {
                let chargeback_tx_id = chargeback.id();
                let disputable_tx = self.get_disputable_transaction(chargeback_tx_id)?;

                if !disputable_tx.is_disputed {
                    return Err(PaymentEngineError::TransactionNotDisputed {
                        client_account: *client_account,
                        tx,
                    })?;
                }

                if disputable_tx.is_deposit() {
                    client_account.unhold(disputable_tx.amount)?;
                } else {
                    client_account.deposit(disputable_tx.amount)?;
                    client_account.unhold(disputable_tx.amount)?;
                }

                client_account.lock();
                disputable_tx.is_disputed = false;
            }
        };

        if let Some(disputable_tx) = Option::<DisputableTransaction>::from(tx) {
            self.disputable_txs.insert(disputable_tx.id, disputable_tx);
        }

        Ok(())
    }

    fn get_disputable_transaction(
        &mut self,
        id: TransactionId,
    ) -> Result<&mut DisputableTransaction, PaymentEngineError> {
        self.disputable_txs
            .get_mut(&id)
            .ok_or(PaymentEngineError::TransactionNotFound { id })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PaymentEngineError {
    #[error("transaction not related to the account tx={tx:?}, account={client_account:?}")]
    UnrelatedTransaction {
        client_account: ClientAccount,
        tx: Transaction,
    },
    #[error("client account locked, cannot process tx={tx:?}, account={client_account:?}")]
    ClientAccountLocked {
        client_account: ClientAccount,
        tx: Transaction,
    },
    #[error("transaction not found id={id:?}")]
    TransactionNotFound { id: TransactionId },
    #[error("transaction already disputed tx={tx:?}, account={client_account:?}")]
    TransactionAlreadyDisputed {
        client_account: ClientAccount,
        tx: Transaction,
    },
    #[error("transaction not disputed tx={tx:?}, account={client_account:?}")]
    TransactionNotDisputed {
        client_account: ClientAccount,
        tx: Transaction,
    },
    #[error(transparent)]
    ClientAccount(#[from] ClientAccountError),
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
