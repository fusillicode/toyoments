use rust_decimal::Decimal;

use crate::transaction::ClientId;

#[derive(Debug, Copy, Clone)]
pub struct ClientAccount {
    pub(in crate::clients_accounts) client_id: ClientId,
    pub(in crate::clients_accounts) available: Decimal,
    pub(in crate::clients_accounts) held: Decimal,
    pub(in crate::clients_accounts) locked: bool,
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
