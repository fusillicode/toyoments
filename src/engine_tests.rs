use std::str::FromStr;

use rust_decimal::Decimal;

use crate::engine::PaymentEngine;
use crate::models::clients_accounts::ClientAccount;
use crate::models::csv_models::Chargeback;
use crate::models::csv_models::ClientId;
use crate::models::csv_models::Deposit;
use crate::models::csv_models::Dispute;
use crate::models::csv_models::PositiveAmount;
use crate::models::csv_models::Resolve;
use crate::models::csv_models::Transaction;
use crate::models::csv_models::TransactionId;
use crate::models::csv_models::Withdrawal;

#[test]
fn handle_transaction_deposit_increases_available() {
    let client_id = ClientId(1);
    let mut client_account = ClientAccount::new(client_id);
    let mut payment_engine = PaymentEngine::new();
    payment_engine
        .handle_transaction(&mut client_account, deposit(client_id, TransactionId(10), "5.50"))
        .unwrap();
    assert_eq!(client_account.available(), dec("5.50"));
    assert_eq!(client_account.held(), Decimal::ZERO);
}

#[test]
fn handle_transaction_withdrawal_reduces_available() {
    let client_id = ClientId(2);
    let mut client_account = ClientAccount::new(client_id);
    let mut payment_engine = PaymentEngine::new();
    payment_engine
        .handle_transaction(&mut client_account, deposit(client_id, TransactionId(1), "10.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, withdrawal(client_id, TransactionId(2), "3.25"))
        .unwrap();
    assert_eq!(client_account.available(), dec("6.75"));
    assert_eq!(client_account.held(), Decimal::ZERO);
}

#[test]
fn handle_transaction_withdrawal_insufficient_funds_errors() {
    let client_id = ClientId(3);
    let mut client_account = ClientAccount::new(client_id);
    let mut payment_engine = PaymentEngine::new();
    let result =
        payment_engine.handle_transaction(&mut client_account, withdrawal(client_id, TransactionId(5), "1.00"));
    assert!(result.is_err());
    assert_eq!(client_account.available(), Decimal::ZERO);
    assert_eq!(client_account.held(), Decimal::ZERO);
}

#[test]
fn handle_transaction_dispute_on_deposit_moves_funds_from_available_to_held() {
    let client_id = ClientId(4);
    let mut client_account = ClientAccount::new(client_id);
    let mut payment_engine = PaymentEngine::new();
    payment_engine
        .handle_transaction(&mut client_account, deposit(client_id, TransactionId(7), "12.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, dispute(client_id, TransactionId(7)))
        .unwrap();
    assert_eq!(client_account.available(), Decimal::ZERO);
    assert_eq!(client_account.held(), dec("12.00"));
}

#[test]
fn handle_transaction_dispute_on_withdrawal_holds_without_reducing_available() {
    let client_id = ClientId(5);
    let mut client_account = ClientAccount::new(client_id);
    let mut payment_engine = PaymentEngine::new();
    payment_engine
        .handle_transaction(&mut client_account, deposit(client_id, TransactionId(8), "10.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, withdrawal(client_id, TransactionId(9), "4.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, dispute(client_id, TransactionId(9)))
        .unwrap();
    assert_eq!(client_account.available(), dec("6.00"));
    assert_eq!(client_account.held(), dec("4.00"));
}

#[test]
fn handle_transaction_resolve_releases_held_into_available() {
    let client_id = ClientId(6);
    let mut client_account = ClientAccount::new(client_id);
    let mut payment_engine = PaymentEngine::new();
    payment_engine
        .handle_transaction(&mut client_account, deposit(client_id, TransactionId(11), "8.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, dispute(client_id, TransactionId(11)))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, resolve(client_id, TransactionId(11)))
        .unwrap();
    assert_eq!(client_account.available(), dec("8.00"));
    assert_eq!(client_account.held(), Decimal::ZERO);
}

#[test]
fn handle_transaction_resolve_without_dispute_errors() {
    let client_id = ClientId(7);
    let mut client_account = ClientAccount::new(client_id);
    let mut payment_engine = PaymentEngine::new();
    payment_engine
        .handle_transaction(&mut client_account, deposit(client_id, TransactionId(12), "3.00"))
        .unwrap();
    let result = payment_engine.handle_transaction(&mut client_account, resolve(client_id, TransactionId(12)));
    assert!(result.is_err());
    assert_eq!(client_account.available(), dec("3.00"));
    assert_eq!(client_account.held(), Decimal::ZERO);
}

#[test]
fn handle_transaction_chargeback_on_deposit_removes_and_locks() {
    let client_id = ClientId(8);
    let mut client_account = ClientAccount::new(client_id);
    let mut payment_engine = PaymentEngine::new();
    payment_engine
        .handle_transaction(&mut client_account, deposit(client_id, TransactionId(13), "15.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, dispute(client_id, TransactionId(13)))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, chargeback(client_id, TransactionId(13)))
        .unwrap();
    assert_eq!(client_account.available(), Decimal::ZERO);
    assert_eq!(client_account.held(), Decimal::ZERO);
    assert!(client_account.locked());
}

#[test]
fn handle_transaction_chargeback_on_withdrawal_restores_and_locks() {
    let client_id = ClientId(9);
    let mut client_account = ClientAccount::new(client_id);
    let mut payment_engine = PaymentEngine::new();
    payment_engine
        .handle_transaction(&mut client_account, deposit(client_id, TransactionId(14), "20.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, withdrawal(client_id, TransactionId(15), "5.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, dispute(client_id, TransactionId(15)))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, chargeback(client_id, TransactionId(15)))
        .unwrap();
    assert_eq!(client_account.available(), dec("20.00"));
    assert_eq!(client_account.held(), Decimal::ZERO);
    assert!(client_account.locked());
}

#[test]
fn handle_transaction_unrelated_client_errors() {
    let client_id = ClientId(10);
    let mut client_account = ClientAccount::new(client_id);
    let mut payment_engine = PaymentEngine::new();
    payment_engine
        .handle_transaction(&mut client_account, deposit(client_id, TransactionId(30), "1.00"))
        .unwrap();
    let mismatched_client_id = ClientId(11);
    let mismatched_deposit = deposit(mismatched_client_id, TransactionId(31), "2.00");
    let result = payment_engine.handle_transaction(&mut client_account, mismatched_deposit);
    assert!(result.is_err());
    assert_eq!(client_account.available(), dec("1.00"));
    assert_eq!(client_account.held(), Decimal::ZERO);
}

#[test]
fn handle_transaction_locked_account_rejects_new_transaction() {
    let client_id = ClientId(12);
    let mut client_account = ClientAccount::new(client_id);
    let mut payment_engine = PaymentEngine::new();
    payment_engine
        .handle_transaction(&mut client_account, deposit(client_id, TransactionId(40), "6.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, dispute(client_id, TransactionId(40)))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, chargeback(client_id, TransactionId(40)))
        .unwrap();
    assert!(client_account.locked());
    let result = payment_engine.handle_transaction(&mut client_account, deposit(client_id, TransactionId(41), "1.00"));
    assert!(result.is_err());
    assert_eq!(client_account.available(), Decimal::ZERO);
    assert_eq!(client_account.held(), Decimal::ZERO);
}

fn deposit(client_id: ClientId, id: TransactionId, amount_str: &str) -> Transaction {
    Transaction::Deposit(Deposit {
        client_id,
        id,
        amount: PositiveAmount::try_from(dec(amount_str)).unwrap(),
    })
}

fn withdrawal(client_id: ClientId, id: TransactionId, amount_str: &str) -> Transaction {
    Transaction::Withdrawal(Withdrawal {
        client_id,
        id,
        amount: PositiveAmount::try_from(dec(amount_str)).unwrap(),
    })
}

fn dispute(client_id: ClientId, id: TransactionId) -> Transaction {
    Transaction::Dispute(Dispute { client_id, id })
}

fn resolve(client_id: ClientId, id: TransactionId) -> Transaction {
    Transaction::Resolve(Resolve { client_id, id })
}

fn chargeback(client_id: ClientId, id: TransactionId) -> Transaction {
    Transaction::Chargeback(Chargeback { client_id, id })
}

fn dec(value: &str) -> Decimal {
    Decimal::from_str(value).unwrap()
}
