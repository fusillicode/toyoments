use std::str::FromStr;

use assert2::let_assert;
use rust_decimal::Decimal;

use crate::account::ClientAccount;
use crate::account::ClientAccountError;
use crate::engine::PaymentEngine;
use crate::engine::payment_engine::PaymentEngineError;
use crate::transaction::Chargeback;
use crate::transaction::ClientId;
use crate::transaction::Deposit;
use crate::transaction::Dispute;
use crate::transaction::PositiveAmount;
use crate::transaction::Resolve;
use crate::transaction::Transaction;
use crate::transaction::TransactionId;
use crate::transaction::Withdrawal;

const TEST_CLIENT_ID: ClientId = ClientId(0);

#[test]
fn handle_transaction_deposit_increases_available() {
    let (mut payment_engine, mut client_account) = setup_engine_and_test_account();
    payment_engine
        .handle_transaction(&mut client_account, deposit(10, "5.50"))
        .unwrap();
    assert_eq!(client_account.available(), dec("5.50"));
    assert_eq!(client_account.held(), Decimal::ZERO);
}

#[test]
fn handle_transaction_withdrawal_reduces_available() {
    let (mut payment_engine, mut client_account) = setup_engine_and_test_account();
    payment_engine
        .handle_transaction(&mut client_account, deposit(1, "10.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, withdrawal(2, "3.25"))
        .unwrap();
    assert_eq!(client_account.available(), dec("6.75"));
    assert_eq!(client_account.held(), Decimal::ZERO);
}

#[test]
fn handle_transaction_dispute_on_deposit_moves_funds_from_available_to_held() {
    let (mut payment_engine, mut client_account) = setup_engine_and_test_account();
    payment_engine
        .handle_transaction(&mut client_account, deposit(7, "12.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, dispute(7))
        .unwrap();
    assert_eq!(client_account.available(), Decimal::ZERO);
    assert_eq!(client_account.held(), dec("12.00"));
}

#[test]
fn handle_transaction_dispute_on_withdrawal_holds_without_reducing_available() {
    let (mut payment_engine, mut client_account) = setup_engine_and_test_account();
    payment_engine
        .handle_transaction(&mut client_account, deposit(8, "10.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, withdrawal(9, "4.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, dispute(9))
        .unwrap();
    assert_eq!(client_account.available(), dec("6.00"));
    assert_eq!(client_account.held(), dec("4.00"));
}

#[test]
fn handle_transaction_resolve_releases_held_into_available() {
    let (mut payment_engine, mut client_account) = setup_engine_and_test_account();
    payment_engine
        .handle_transaction(&mut client_account, deposit(11, "8.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, dispute(11))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, resolve(11))
        .unwrap();
    assert_eq!(client_account.available(), dec("8.00"));
    assert_eq!(client_account.held(), Decimal::ZERO);
}

#[test]
fn handle_transaction_chargeback_on_deposit_removes_and_locks() {
    let (mut payment_engine, mut client_account) = setup_engine_and_test_account();
    payment_engine
        .handle_transaction(&mut client_account, deposit(13, "15.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, dispute(13))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, chargeback(13))
        .unwrap();
    assert_eq!(client_account.available(), Decimal::ZERO);
    assert_eq!(client_account.held(), Decimal::ZERO);
    assert!(client_account.is_locked());
}

#[test]
fn handle_transaction_chargeback_on_withdrawal_restores_and_locks() {
    let (mut payment_engine, mut client_account) = setup_engine_and_test_account();
    payment_engine
        .handle_transaction(&mut client_account, deposit(14, "20.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, withdrawal(15, "5.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, dispute(15))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, chargeback(15))
        .unwrap();
    assert_eq!(client_account.available(), dec("20.00"));
    assert_eq!(client_account.held(), Decimal::ZERO);
    assert!(client_account.is_locked());
}

#[test]
fn handle_transaction_of_another_client_errors_as_expected() {
    let (mut payment_engine, mut client_account) = setup_engine_and_test_account();
    payment_engine
        .handle_transaction(&mut client_account, deposit(30, "1.00"))
        .unwrap();
    let mismatched_client_id = ClientId(TEST_CLIENT_ID.0 + 1);
    let mismatched_deposit = Transaction::Deposit(Deposit {
        client_id: mismatched_client_id,
        id: TransactionId(31),
        amount: PositiveAmount::try_from(dec("2.00")).unwrap(),
    });

    let res = payment_engine.handle_transaction(&mut client_account, mismatched_deposit);

    let_assert!(
        Err(PaymentEngineError::UnrelatedTransaction {
            client_account: err_account,
            tx
        }) = res
    );
    assert_eq!(err_account.client_id(), TEST_CLIENT_ID);
    assert_eq!(tx.client_id(), mismatched_client_id);
    assert_eq!(tx.id(), TransactionId(31));
    assert_eq!(client_account.available(), dec("1.00"));
    assert_eq!(client_account.held(), Decimal::ZERO);
}

#[test]
fn handle_transaction_withdrawal_with_insufficient_funds_errors_as_expected() {
    let (mut payment_engine, mut client_account) = setup_engine_and_test_account();

    let res = payment_engine.handle_transaction(&mut client_account, withdrawal(5, "1.00"));

    let_assert!(
        Err(PaymentEngineError::ClientAccount(
            ClientAccountError::InsufficientFunds {
                client_account: err_account,
                amount
            }
        )) = res
    );
    assert_eq!(err_account.client_id(), TEST_CLIENT_ID);
    assert_eq!(amount.as_inner(), dec("1.00"));
    assert_eq!(client_account.available(), Decimal::ZERO);
    assert_eq!(client_account.held(), Decimal::ZERO);
}

#[test]
fn handle_transaction_dispute_same_transaction_twice_errors_as_expected() {
    let (mut payment_engine, mut client_account) = setup_engine_and_test_account();
    payment_engine
        .handle_transaction(&mut client_account, deposit(50, "5.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, dispute(50))
        .unwrap();

    let res = payment_engine.handle_transaction(&mut client_account, dispute(50));

    let_assert!(
        Err(PaymentEngineError::TransactionAlreadyDisputed {
            client_account: err_account,
            tx
        }) = res
    );
    assert_eq!(err_account.client_id(), TEST_CLIENT_ID);
    assert_eq!(tx.id(), TransactionId(50));
}

#[test]
fn handle_transaction_resolve_without_dispute_errors_as_expected() {
    let (mut payment_engine, mut client_account) = setup_engine_and_test_account();
    payment_engine
        .handle_transaction(&mut client_account, deposit(12, "3.00"))
        .unwrap();

    let res = payment_engine.handle_transaction(&mut client_account, resolve(12));

    let_assert!(
        Err(PaymentEngineError::TransactionNotDisputed {
            client_account: err_account,
            tx
        }) = res
    );
    assert_eq!(err_account.client_id(), TEST_CLIENT_ID);
    assert_eq!(tx.id(), TransactionId(12));
    assert_eq!(client_account.available(), dec("3.00"));
    assert_eq!(client_account.held(), Decimal::ZERO);
}

#[test]
fn handle_transaction_resolve_unknown_transaction_errors_as_expected() {
    let (mut payment_engine, mut client_account) = setup_engine_and_test_account();

    let res = payment_engine.handle_transaction(&mut client_account, resolve(999));

    let_assert!(Err(PaymentEngineError::TransactionNotFound { id }) = res);
    assert_eq!(id, TransactionId(999));
}

#[test]
fn handle_transaction_on_locked_account_errors_as_expected() {
    let (mut payment_engine, mut client_account) = setup_engine_and_test_account();
    payment_engine
        .handle_transaction(&mut client_account, deposit(40, "6.00"))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, dispute(40))
        .unwrap();
    payment_engine
        .handle_transaction(&mut client_account, chargeback(40))
        .unwrap();
    assert!(client_account.is_locked());

    let res = payment_engine.handle_transaction(&mut client_account, deposit(41, "1.00"));

    let_assert!(
        Err(PaymentEngineError::ClientAccountLocked {
            client_account: err_account,
            tx
        }) = res
    );
    assert_eq!(err_account.client_id(), TEST_CLIENT_ID);
    assert_eq!(tx.id(), TransactionId(41));
    assert_eq!(client_account.available(), Decimal::ZERO);
    assert_eq!(client_account.held(), Decimal::ZERO);
}

#[test]
fn handle_transaction_dispute_cross_client_without_override_errors_as_expected() {
    let mut payment_engine = PaymentEngine::default();
    // Victim client 0 deposit id=80
    let mut victim_account = ClientAccount::new(TEST_CLIENT_ID);
    payment_engine
        .handle_transaction(&mut victim_account, deposit(80, "9.00"))
        .unwrap();

    // Attacker client 1 disputes victim's transaction id=80 -> now simply not found for that client
    let attacker_client_id = ClientId(TEST_CLIENT_ID.0 + 1);
    let mut attacker_account = ClientAccount::new(attacker_client_id);
    let attacker_dispute = dispute_for(attacker_client_id, 80);

    let res = payment_engine.handle_transaction(&mut attacker_account, attacker_dispute);

    let_assert!(Err(PaymentEngineError::TransactionNotFound { id }) = res);
    assert_eq!(id, TransactionId(80));
    // Victim account intact
    assert_eq!(victim_account.available(), dec("9.00"));
    assert_eq!(victim_account.held(), Decimal::ZERO);
    // Attacker unchanged
    assert_eq!(attacker_account.available(), Decimal::ZERO);
    assert_eq!(attacker_account.held(), Decimal::ZERO);
}

#[test]
fn handle_transaction_dispute_same_tx_id_different_clients_are_isolated() {
    let mut payment_engine = PaymentEngine::default();
    // Client 0 deposit tx=70
    let mut client_account_0 = ClientAccount::new(TEST_CLIENT_ID);
    payment_engine
        .handle_transaction(&mut client_account_0, deposit(70, "5.00"))
        .unwrap();

    // Client 1 deposit with SAME tx id=70 (allowed; separate namespace)
    let client1_id = ClientId(TEST_CLIENT_ID.0 + 1);
    let mut client_account_1 = ClientAccount::new(client1_id);
    let other_deposit = deposit_for(client1_id, 70, "7.50");
    payment_engine
        .handle_transaction(&mut client_account_1, other_deposit)
        .unwrap();

    // Both can independently dispute their own tx id=70
    payment_engine
        .handle_transaction(&mut client_account_0, dispute(70))
        .unwrap();
    let client1_dispute = dispute_for(client1_id, 70);
    payment_engine
        .handle_transaction(&mut client_account_1, client1_dispute)
        .unwrap();

    // Balances reflect held amounts separately
    assert_eq!(client_account_0.available(), Decimal::ZERO);
    assert_eq!(client_account_0.held(), dec("5.00"));
    assert_eq!(client_account_1.available(), Decimal::ZERO);
    assert_eq!(client_account_1.held(), dec("7.50"));
}

fn setup_engine_and_test_account() -> (PaymentEngine, ClientAccount) {
    (PaymentEngine::default(), ClientAccount::new(TEST_CLIENT_ID))
}

fn deposit(transaction_id: u32, amount: &str) -> Transaction {
    deposit_for(TEST_CLIENT_ID, transaction_id, amount)
}

fn deposit_for(client_id: ClientId, transaction_id: u32, amount: &str) -> Transaction {
    Transaction::Deposit(Deposit {
        client_id,
        id: TransactionId(transaction_id),
        amount: PositiveAmount::try_from(dec(amount)).unwrap(),
    })
}

fn withdrawal(transaction_id: u32, amount: &str) -> Transaction {
    Transaction::Withdrawal(Withdrawal {
        client_id: TEST_CLIENT_ID,
        id: TransactionId(transaction_id),
        amount: PositiveAmount::try_from(dec(amount)).unwrap(),
    })
}

fn dispute(transaction_id: u32) -> Transaction {
    Transaction::Dispute(Dispute {
        client_id: TEST_CLIENT_ID,
        id: TransactionId(transaction_id),
    })
}

fn dispute_for(client_id: ClientId, transaction_id: u32) -> Transaction {
    Transaction::Dispute(Dispute {
        client_id,
        id: TransactionId(transaction_id),
    })
}

fn resolve(transaction_id: u32) -> Transaction {
    Transaction::Resolve(Resolve {
        client_id: TEST_CLIENT_ID,
        id: TransactionId(transaction_id),
    })
}

fn chargeback(transaction_id: u32) -> Transaction {
    Transaction::Chargeback(Chargeback {
        client_id: TEST_CLIENT_ID,
        id: TransactionId(transaction_id),
    })
}

fn dec(value: &str) -> Decimal {
    Decimal::from_str(value).unwrap()
}
