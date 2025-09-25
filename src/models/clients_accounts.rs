use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::models::csv_models::ClientId;
use crate::models::csv_models::PositiveAmount;

pub struct ClientsAccounts(HashMap<ClientId, ClientAccount>);

impl ClientsAccounts {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get_or_create_new_account(&mut self, client_id: ClientId) -> &mut ClientAccount {
        self.0
            .entry(client_id)
            .or_insert(ClientAccount::new(client_id))
    }

    pub fn as_inner(&self) -> &HashMap<ClientId, ClientAccount> {
        &self.0
    }
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

    pub fn client_id(&self) -> ClientId {
        self.client_id
    }

    pub fn available(&self) -> Decimal {
        self.available
    }

    pub fn held(&self) -> Decimal {
        self.held
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    pub fn total(&self) -> Decimal {
        self.available + self.held
    }

    pub fn lock(&mut self) {
        self.locked = true;
    }

    pub fn deposit(&mut self, amount: PositiveAmount) -> Result<(), ClientAccountError> {
        let new_available = self.available.checked_add(amount.as_inner()).ok_or(
            ClientAccountError::OperationOverflow {
                client_account: *self,
                amount,
            },
        )?;
        self.available = new_available;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: PositiveAmount) -> Result<(), ClientAccountError> {
        if self.available < amount.as_inner() {
            return Err(ClientAccountError::InsufficientFunds {
                client_account: *self,
                amount,
            });
        }
        let Some(new_available) = self.available.checked_sub(amount.as_inner()) else {
            return Err(ClientAccountError::OperationOverflow {
                client_account: *self,
                amount,
            });
        };
        self.available = new_available;
        Ok(())
    }

    pub fn withdraw_and_hold(&mut self, amount: PositiveAmount) -> Result<(), ClientAccountError> {
        if self.available < amount.as_inner() {
            return Err(ClientAccountError::InsufficientFunds {
                client_account: *self,
                amount,
            });
        }
        let new_available = self.available.checked_sub(amount.as_inner()).ok_or(
            ClientAccountError::OperationOverflow {
                client_account: *self,
                amount,
            },
        )?;
        let new_held = self.held.checked_add(amount.as_inner()).ok_or(
            ClientAccountError::OperationOverflow {
                client_account: *self,
                amount,
            },
        )?;
        self.available = new_available;
        self.held = new_held;
        Ok(())
    }

    pub fn hold(&mut self, amount: PositiveAmount) -> Result<(), ClientAccountError> {
        let Some(new_held) = self.held.checked_add(amount.as_inner()) else {
            return Err(ClientAccountError::OperationOverflow {
                client_account: *self,
                amount,
            });
        };
        self.held = new_held;
        Ok(())
    }

    pub fn unhold(&mut self, amount: PositiveAmount) -> Result<(), ClientAccountError> {
        if self.held < amount.as_inner() {
            return Err(ClientAccountError::InsufficientFunds {
                client_account: *self,
                amount,
            });
        }
        let new_held = self.held.checked_sub(amount.as_inner()).ok_or({
            ClientAccountError::OperationOverflow {
                client_account: *self,
                amount,
            }
        })?;
        self.held = new_held;
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ClientAccountError {
    #[error("operation overflow applying amount={amount:?} to account={client_account:?}")]
    OperationOverflow {
        client_account: ClientAccount,
        amount: PositiveAmount,
    },
    #[error("insufficient funds amount={amount:?} account={client_account:?}")]
    InsufficientFunds {
        client_account: ClientAccount,
        amount: PositiveAmount,
    },
}
