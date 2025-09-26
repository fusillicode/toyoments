use crate::transaction::PositiveAmount;
use crate::transaction::Transaction;
use crate::transaction::TransactionId;

#[derive(Debug)]
pub struct DisputableTransaction {
    pub(in crate::engine) id: TransactionId,
    pub(in crate::engine) amount: PositiveAmount,
    pub(in crate::engine) is_disputed: bool,
    pub(in crate::engine) kind: DisputableTransactionKind,
}

impl DisputableTransaction {
    pub const fn is_deposit(&self) -> bool {
        self.kind.is_deposit()
    }
}

impl From<Transaction> for Option<DisputableTransaction> {
    fn from(tx: Transaction) -> Self {
        let id = tx.id();
        match tx {
            Transaction::Deposit(deposit) => Some(DisputableTransaction {
                id,
                amount: deposit.amount,
                is_disputed: false,
                kind: DisputableTransactionKind::Deposit,
            }),
            Transaction::Withdrawal(withdrawal) => Some(DisputableTransaction {
                id,
                amount: withdrawal.amount,
                is_disputed: false,
                kind: DisputableTransactionKind::Withdrawal,
            }),
            Transaction::Dispute(_) | Transaction::Resolve(_) | Transaction::Chargeback(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(in crate::engine) enum DisputableTransactionKind {
    Deposit,
    Withdrawal,
}

impl DisputableTransactionKind {
    const fn is_deposit(self) -> bool {
        match self {
            Self::Deposit => true,
            Self::Withdrawal => false,
        }
    }
}
