//! Free functions that mutate a supplied [`ClientAccount`].
//!
//! Rationale:
//! Originally these were [`ClientAccount`] methods; they were extracted to emphasize a
//! clear separation between the account's data model and the business operations that mutate it.
//! This makes it easier to audit side effects, reason about invariants, and (if desired) mock
//! or wrap mutation logic independently of the data container.
//!
//! Alternatives:
//! - Keep inherent methods but split the impl across files, one with read-only accessors, one with mutating methods.
//! - Introduce a "manager" abstraction (e.g. `ClientAccountManager`) that owns or borrows a `ClientAccount` and permits
//!   to mutate it.
//!
//! These functions intentionally accept `&mut ClientAccount` so that the caller
//! must make mutability explicit at the call site.

use rust_decimal::Decimal;

use crate::account::ClientAccount;
use crate::transaction::PositiveAmount;

#[derive(thiserror::Error, Debug)]
pub enum ClientAccountError {
    #[error("overflow while applying {amount} to {client_account}")]
    OperationOverflow {
        client_account: ClientAccount,
        amount: PositiveAmount,
    },
    #[error("insufficient available funds, need {amount} in {client_account}")]
    InsufficientFunds {
        client_account: ClientAccount,
        amount: PositiveAmount,
    },
}

/// Adds `amount` to the account's available funds.
///
/// # Errors
///
/// Returns an error if:
/// - Adding `amount` to available funds overflows ([`ClientAccountError::OperationOverflow`]).
pub fn deposit(client_account: &mut ClientAccount, amount: PositiveAmount) -> Result<(), ClientAccountError> {
    client_account.available = checked_add_to_available(client_account, amount)?;
    Ok(())
}

/// Subtracts `amount` from the account's available funds.
///
/// # Errors
///
/// Returns an error if:
/// - Available funds are less than `amount` ([`ClientAccountError::InsufficientFunds`]).
/// - Subtracting `amount` from available funds overflows ([`ClientAccountError::OperationOverflow`]).
pub fn withdraw(client_account: &mut ClientAccount, amount: PositiveAmount) -> Result<(), ClientAccountError> {
    client_account.available = checked_sub_from_available(client_account, amount)?;
    Ok(())
}

/// Moves `amount` from external context into the held funds bucket (no available subtraction here).
///
/// # Errors
///
/// Returns an error if:
/// - Adding `amount` to held funds overflows ([`ClientAccountError::OperationOverflow`]).
pub fn hold(client_account: &mut ClientAccount, amount: PositiveAmount) -> Result<(), ClientAccountError> {
    client_account.held = checked_add_to_held(client_account, amount)?;
    Ok(())
}

/// Decreases held funds by `amount`.
///
/// # Errors
///
/// Returns an error if:
/// - Held funds are less than `amount` ([`ClientAccountError::InsufficientFunds`]).
/// - Subtracting `amount` from held funds overflows ([`ClientAccountError::OperationOverflow`]).
pub fn unhold(client_account: &mut ClientAccount, amount: PositiveAmount) -> Result<(), ClientAccountError> {
    client_account.held = checked_sub_from_held(client_account, amount)?;
    Ok(())
}

/// Locks the supplied [`ClientAccount`].
///
/// Sets its `locked` flag to `true`, preventing further balance mutations that
/// require an unlocked account.
/// Idempotent: calling again has no additional effect.
pub const fn lock(client_account: &mut ClientAccount) {
    client_account.locked = true;
}

/// Atomically subtracts `amount` from available and increases held by the same `amount`.
/// Used when disputing a deposit.
///
/// # Errors
///
/// Returns an error if:
/// - Available funds are less than `amount` ([`ClientAccountError::InsufficientFunds`]).
/// - Adjusting available or held funds overflows ([`ClientAccountError::OperationOverflow`]).
pub fn withdraw_and_hold(client_account: &mut ClientAccount, amount: PositiveAmount) -> Result<(), ClientAccountError> {
    let new_available = checked_sub_from_available(client_account, amount)?;
    let new_held = checked_add_to_held(client_account, amount)?;
    client_account.available = new_available;
    client_account.held = new_held;
    Ok(())
}

/// Moves `amount` from held back to available funds.
/// Used when resolving a dispute on a deposit.
///
/// # Errors
///
/// Returns an error if:
/// - Held funds are less than `amount` ([`ClientAccountError::InsufficientFunds`]).
/// - Adjusting available or held funds overflows ([`ClientAccountError::OperationOverflow`]).
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

const fn overflow_error(client_account: &ClientAccount, amount: PositiveAmount) -> ClientAccountError {
    ClientAccountError::OperationOverflow {
        client_account: *client_account,
        amount,
    }
}

const fn insufficient_funds_error(client_account: &ClientAccount, amount: PositiveAmount) -> ClientAccountError {
    ClientAccountError::InsufficientFunds {
        client_account: *client_account,
        amount,
    }
}
