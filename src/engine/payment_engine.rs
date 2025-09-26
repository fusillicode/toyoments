use std::collections::HashMap;

use crate::account::ClientAccount;
use crate::account::ClientAccountError;
use crate::engine::disputable::DisputableTransaction;
use crate::transaction::Transaction;
use crate::transaction::TransactionId;

#[cfg(test)]
#[path = "tests/payment_engine_tests.rs"]
mod payment_engine_tests;

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

        if client_account.is_locked() {
            return Err(PaymentEngineError::ClientAccountLocked {
                client_account: *client_account,
                tx,
            })?;
        }

        match tx {
            Transaction::Deposit(dep) => crate::account::deposit(client_account, dep.amount)?,
            Transaction::Withdrawal(wd) => crate::account::withdraw(client_account, wd.amount)?,
            Transaction::Dispute(dispute) => {
                let disputed_tx_id = dispute.id;
                let disputable_tx = self.get_disputable_transaction(disputed_tx_id)?;

                if disputable_tx.is_disputed {
                    return Err(PaymentEngineError::TransactionAlreadyDisputed {
                        client_account: *client_account,
                        tx,
                    })?;
                }

                if disputable_tx.is_deposit() {
                    crate::account::withdraw_and_hold(client_account, disputable_tx.amount)?;
                } else {
                    crate::account::hold(client_account, disputable_tx.amount)?;
                }

                disputable_tx.is_disputed = true;
            }
            Transaction::Resolve(resolve) => {
                let resolvable_tx_id = resolve.id;
                let disputable_tx = self.get_disputable_transaction(resolvable_tx_id)?;

                if !disputable_tx.is_disputed {
                    return Err(PaymentEngineError::TransactionNotDisputed {
                        client_account: *client_account,
                        tx,
                    })?;
                }

                crate::account::unhold_and_deposit(client_account, disputable_tx.amount)?;

                disputable_tx.is_disputed = false;
            }
            Transaction::Chargeback(chargeback) => {
                let chargeback_tx_id = chargeback.id;
                let disputable_tx = self.get_disputable_transaction(chargeback_tx_id)?;

                if !disputable_tx.is_disputed {
                    return Err(PaymentEngineError::TransactionNotDisputed {
                        client_account: *client_account,
                        tx,
                    })?;
                }

                if disputable_tx.is_deposit() {
                    crate::account::unhold(client_account, disputable_tx.amount)?;
                } else {
                    crate::account::deposit_and_unhold(client_account, disputable_tx.amount)?;
                }
                crate::account::lock(client_account);

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
