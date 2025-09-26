use rust_decimal::Decimal;

use crate::clients_accounts::client_account::ClientAccount;
use crate::transaction::PositiveAmount;

pub fn lock(client_account: &mut ClientAccount) {
    client_account.locked = true;
}

pub fn deposit(client_account: &mut ClientAccount, amount: PositiveAmount) -> Result<(), ClientAccountError> {
    client_account.available = checked_add_to_available(client_account, amount)?;
    Ok(())
}

pub fn withdraw(client_account: &mut ClientAccount, amount: PositiveAmount) -> Result<(), ClientAccountError> {
    client_account.available = checked_sub_from_available(client_account, amount)?;
    Ok(())
}

pub fn hold(client_account: &mut ClientAccount, amount: PositiveAmount) -> Result<(), ClientAccountError> {
    client_account.held = checked_add_to_held(client_account, amount)?;
    Ok(())
}

pub fn unhold(client_account: &mut ClientAccount, amount: PositiveAmount) -> Result<(), ClientAccountError> {
    client_account.held = checked_sub_from_held(client_account, amount)?;
    Ok(())
}

pub fn withdraw_and_hold(client_account: &mut ClientAccount, amount: PositiveAmount) -> Result<(), ClientAccountError> {
    let new_available = checked_sub_from_available(client_account, amount)?;
    let new_held = checked_add_to_held(client_account, amount)?;
    client_account.available = new_available;
    client_account.held = new_held;
    Ok(())
}

pub fn unhold_and_deposit(
    client_account: &mut ClientAccount,
    amount: PositiveAmount,
) -> Result<(), ClientAccountError> {
    let new_held = checked_sub_from_held(client_account, amount)?;
    let new_available = checked_add_to_available(client_account, amount)?;
    client_account.held = new_held;
    client_account.available = new_available;
    Ok(())
}

pub fn deposit_and_unhold(
    client_account: &mut ClientAccount,
    amount: PositiveAmount,
) -> Result<(), ClientAccountError> {
    let new_available = checked_add_to_available(client_account, amount)?;
    let new_held = checked_sub_from_held(client_account, amount)?;
    client_account.available = new_available;
    client_account.held = new_held;
    Ok(())
}

fn checked_add_to_available(
    client_account: &ClientAccount,
    amount: PositiveAmount,
) -> Result<Decimal, ClientAccountError> {
    client_account
        .available
        .checked_add(amount.as_inner())
        .ok_or_else(|| overflow_error(client_account, amount))
}

fn checked_sub_from_available(
    client_account: &ClientAccount,
    amount: PositiveAmount,
) -> Result<Decimal, ClientAccountError> {
    if client_account.available < amount.as_inner() {
        return Err(insufficient_funds_error(client_account, amount));
    }
    client_account
        .available
        .checked_sub(amount.as_inner())
        .ok_or_else(|| overflow_error(client_account, amount))
}

fn checked_add_to_held(client_account: &ClientAccount, amount: PositiveAmount) -> Result<Decimal, ClientAccountError> {
    client_account
        .held
        .checked_add(amount.as_inner())
        .ok_or_else(|| overflow_error(client_account, amount))
}

fn checked_sub_from_held(
    client_account: &ClientAccount,
    amount: PositiveAmount,
) -> Result<Decimal, ClientAccountError> {
    if client_account.held < amount.as_inner() {
        return Err(insufficient_funds_error(client_account, amount));
    }
    client_account
        .held
        .checked_sub(amount.as_inner())
        .ok_or_else(|| overflow_error(client_account, amount))
}

fn overflow_error(client_account: &ClientAccount, amount: PositiveAmount) -> ClientAccountError {
    ClientAccountError::OperationOverflow {
        client_account: *client_account,
        amount,
    }
}

fn insufficient_funds_error(client_account: &ClientAccount, amount: PositiveAmount) -> ClientAccountError {
    ClientAccountError::InsufficientFunds {
        client_account: *client_account,
        amount,
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
