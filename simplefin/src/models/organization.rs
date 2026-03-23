use serde::{Deserialize, Serialize};

/// Description of the financial institution that owns a SimpleFIN account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    /// Bridge URL for the organization.
    #[serde(rename = "sfin-url")]
    pub sfin_url: String,

    /// Domain name associated with the organization, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,

    /// Human-friendly organization name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Public website for the organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Organization identifier included by the provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl Organization {
    /// Returns a display-friendly name for the organization.
    pub fn display_name(&self) -> &str {
        self.name
            .as_deref()
            .or(self.domain.as_deref())
            .or(self.id.as_deref())
            .unwrap_or(&self.sfin_url)
    }

    /// Returns a key suitable for deduplication.
    pub fn key(&self) -> &str {
        self.id
            .as_deref()
            .or(self.domain.as_deref())
            .unwrap_or(&self.sfin_url)
    }
}
