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
        self.0.entry(client_id).or_insert(ClientAccount::new(client_id))
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
    pub fn new(client_id: ClientId) -> Self {
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
        self.available = self.checked_add_to_available(amount)?;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: PositiveAmount) -> Result<(), ClientAccountError> {
        self.available = self.checked_sub_from_available(amount)?;
        Ok(())
    }

    pub fn hold(&mut self, amount: PositiveAmount) -> Result<(), ClientAccountError> {
        self.held = self.checked_add_to_held(amount)?;
        Ok(())
    }

    pub fn unhold(&mut self, amount: PositiveAmount) -> Result<(), ClientAccountError> {
        self.held = self.checked_sub_from_held(amount)?;
        Ok(())
    }

    pub fn withdraw_and_hold(&mut self, amount: PositiveAmount) -> Result<(), ClientAccountError> {
        let new_available = self.checked_sub_from_available(amount)?;
        let new_held = self.checked_add_to_held(amount)?;
        self.available = new_available;
        self.held = new_held;
        Ok(())
    }

    pub fn unhold_and_deposit(&mut self, amount: PositiveAmount) -> Result<(), ClientAccountError> {
        let new_held = self.checked_sub_from_held(amount)?;
        let new_available = self.checked_add_to_available(amount)?;
        self.held = new_held;
        self.available = new_available;
        Ok(())
    }

    pub fn deposit_and_unhold(&mut self, amount: PositiveAmount) -> Result<(), ClientAccountError> {
        let new_available = self.checked_add_to_available(amount)?;
        let new_held = self.checked_sub_from_held(amount)?;
        self.available = new_available;
        self.held = new_held;
        Ok(())
    }

    fn checked_add_to_available(&self, amount: PositiveAmount) -> Result<Decimal, ClientAccountError> {
        self.available
            .checked_add(amount.as_inner())
            .ok_or_else(|| self.overflow_error(amount))
    }

    fn checked_sub_from_available(&self, amount: PositiveAmount) -> Result<Decimal, ClientAccountError> {
        if self.available < amount.as_inner() {
            return Err(self.insufficient_funds_error(amount));
        }
        self.available
            .checked_sub(amount.as_inner())
            .ok_or_else(|| self.overflow_error(amount))
    }

    fn checked_add_to_held(&self, amount: PositiveAmount) -> Result<Decimal, ClientAccountError> {
        self.held
            .checked_add(amount.as_inner())
            .ok_or_else(|| self.overflow_error(amount))
    }

    fn checked_sub_from_held(&self, amount: PositiveAmount) -> Result<Decimal, ClientAccountError> {
        if self.held < amount.as_inner() {
            return Err(self.insufficient_funds_error(amount));
        }
        self.held
            .checked_sub(amount.as_inner())
            .ok_or_else(|| self.overflow_error(amount))
    }

    fn overflow_error(&self, amount: PositiveAmount) -> ClientAccountError {
        ClientAccountError::OperationOverflow {
            client_account: *self,
            amount,
        }
    }

    fn insufficient_funds_error(&self, amount: PositiveAmount) -> ClientAccountError {
        ClientAccountError::InsufficientFunds {
            client_account: *self,
            amount,
        }
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
