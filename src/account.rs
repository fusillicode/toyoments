use std::collections::HashMap;

use crate::transaction::ClientId;

pub mod client_account;
pub mod client_account_ops;

pub use client_account::ClientAccount;
pub use client_account_ops::ClientAccountError;
pub use client_account_ops::deposit;
pub use client_account_ops::deposit_and_unhold;
pub use client_account_ops::hold;
pub use client_account_ops::lock;
pub use client_account_ops::unhold;
pub use client_account_ops::unhold_and_deposit;
pub use client_account_ops::withdraw;
pub use client_account_ops::withdraw_and_hold;

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
