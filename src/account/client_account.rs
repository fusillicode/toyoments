use rust_decimal::Decimal;

use crate::transaction::ClientId;

#[derive(Debug, Copy, Clone)]
pub struct ClientAccount {
    pub(in crate::account) client_id: ClientId,
    pub(in crate::account) available: Decimal,
    pub(in crate::account) held: Decimal,
    pub(in crate::account) locked: bool,
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

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    pub fn total(&self) -> Decimal {
        self.available + self.held
    }
}
