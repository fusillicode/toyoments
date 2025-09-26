use std::collections::HashMap;

use crate::transaction::ClientId;

pub mod model;
pub mod ops;

pub use model::ClientAccount;
pub use ops::ClientAccountError;
pub use ops::deposit;
pub use ops::deposit_and_unhold;
pub use ops::hold;
pub use ops::lock;
pub use ops::unhold;
pub use ops::unhold_and_deposit;
pub use ops::withdraw;
pub use ops::withdraw_and_hold;

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
