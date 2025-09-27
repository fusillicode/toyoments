//! Transaction processing engine.
//!
//! Provides [`PaymentEngine`] which applies incoming [`crate::transaction::Transaction`]s,
//! tracks disputable state, and mutates client accounts via [`crate::account`] helpers.
//! [`disputable_transaction`] private module provides the tracking of disputable transaction.

mod disputable_transaction;
pub mod payment_engine;

pub use payment_engine::PaymentEngine;
