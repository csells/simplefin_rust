use serde::{Deserialize, Serialize};

use super::account::Account;

/// Structured response returned by the SimpleFIN `/accounts` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSet {
    /// Informational messages reported by the bridge.
    ///
    /// Named "errors" in the wire format despite containing informational messages,
    /// not necessarily errors.
    #[serde(rename = "errors")]
    pub server_messages: Vec<String>,

    /// Collection of accounts returned by the server.
    pub accounts: Vec<Account>,
}

impl AccountSet {
    /// Returns a new `AccountSet` containing only accounts that belong to
    /// the specified organization ID.
    pub fn filter_by_organization_id(&self, org_id: &str) -> AccountSet {
        let filtered = self
            .accounts
            .iter()
            .filter(|account| {
                account
                    .org
                    .id
                    .as_deref()
                    .is_some_and(|id| id == org_id)
            })
            .cloned()
            .collect();

        AccountSet {
            server_messages: self.server_messages.clone(),
            accounts: filtered,
        }
    }
}
