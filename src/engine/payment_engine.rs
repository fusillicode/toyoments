use std::collections::HashMap;

use crate::account::ClientAccount;
use crate::account::ClientAccountError;
use crate::engine::disputable_transaction::DisputableTransaction;
use crate::transaction::ClientId;
use crate::transaction::Transaction;
use crate::transaction::TransactionId;

#[cfg(test)]
#[path = "./tests/payment_engine_tests.rs"]
mod payment_engine_tests;

#[derive(Default)]
pub struct PaymentEngine {
    /// Disputable transactions indexed by [`ClientId`] and [`TransactionId`] to
    /// prevent cross‑client overwrites or denial-of-dispute scenarios.
    disputable_txs: HashMap<(ClientId, TransactionId), DisputableTransaction>,
}

impl PaymentEngine {
    /// Processes a single transaction by mutating the provided [`ClientAccount`].
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The transaction refers to an account that is not the one supplied
    ///   ([`PaymentEngineError::UnrelatedTransaction`]).
    /// - The account is locked ([`PaymentEngineError::ClientAccountLocked`]).
    /// - A dispute action references a transaction that does not exist ([`PaymentEngineError::TransactionNotFound`]).
    /// - A dispute is initiated on an already disputed transaction
    ///   ([`PaymentEngineError::TransactionAlreadyDisputed`]).
    /// - A resolve or chargeback targets a transaction not currently disputed
    ///   ([`PaymentEngineError::TransactionNotDisputed`]).
    /// - An underlying account funds operation fails (wrapped in [`PaymentEngineError::ClientAccount`]).
    pub fn handle_transaction(
        &mut self,
        client_account: &mut ClientAccount,
        tx: Transaction,
    ) -> Result<(), PaymentEngineError> {
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
                let disputable_tx = self.get_disputable_transaction(client_account.client_id(), disputed_tx_id)?;

                if disputable_tx.is_disputed {
                    return Err(PaymentEngineError::TransactionAlreadyDisputed {
                        client_account: *client_account,
                        tx,
                    })?;
                }

                // Deposit dispute: move funds from available to held (freeze spendability)
                if disputable_tx.is_deposit() {
                    crate::account::withdraw_and_hold(client_account, disputable_tx.amount)?;
                }
                // Withdrawal dispute (symmetric freeze model): no immediate balance mutation.
                // We only mark it disputed; resolution or chargeback will decide funds.

                disputable_tx.is_disputed = true;
            }
            Transaction::Resolve(resolve) => {
                let resolvable_tx_id = resolve.id;
                let disputable_tx = self.get_disputable_transaction(client_account.client_id(), resolvable_tx_id)?;

                if !disputable_tx.is_disputed {
                    return Err(PaymentEngineError::TransactionNotDisputed {
                        client_account: *client_account,
                        tx,
                    })?;
                }

                if disputable_tx.is_deposit() {
                    // Resolving a disputed deposit: release held back to available.
                    crate::account::unhold_and_deposit(client_account, disputable_tx.amount)?;
                } else {
                    // Resolving a disputed withdrawal: refund (re-credit) the amount now.
                    // Original withdrawal already reduced available; a dispute froze it logically.
                    crate::account::deposit(client_account, disputable_tx.amount)?;
                }

                disputable_tx.is_disputed = false;
            }
            Transaction::Chargeback(chargeback) => {
                let chargeback_tx_id = chargeback.id;
                let disputable_tx = self.get_disputable_transaction(client_account.client_id(), chargeback_tx_id)?;

                if !disputable_tx.is_disputed {
                    return Err(PaymentEngineError::TransactionNotDisputed {
                        client_account: *client_account,
                        tx,
                    })?;
                }

                // Chargeback of a deposit: permanently remove held funds.
                if disputable_tx.is_deposit() {
                    crate::account::unhold(client_account, disputable_tx.amount)?;
                }
                // Chargeback of a withdrawal: do NOT refund; withdrawal stands, but lock account.
                crate::account::lock(client_account);

                disputable_tx.is_disputed = false;
            }
        }

        if let Some(disputable_tx) = Option::<DisputableTransaction>::from(tx) {
            let key = (disputable_tx.client_id, disputable_tx.id);
            self.disputable_txs.insert(key, disputable_tx);
        }

        Ok(())
    }

    fn get_disputable_transaction(
        &mut self,
        client_id: ClientId,
        id: TransactionId,
    ) -> Result<&mut DisputableTransaction, PaymentEngineError> {
        self.disputable_txs
            .get_mut(&(client_id, id))
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
