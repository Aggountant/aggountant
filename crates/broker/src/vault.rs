//! O-24 module: **cred**. The `Vault` holds authority credentials and hands out
//! opaque [`CredentialHandle`]s. A handle is *not* the secret: it derefs to a
//! service label and an in-memory secret an adapter can present to its transport,
//! but it never `Serialize`s and its `Debug` never prints the secret — so a
//! credential cannot leak through a log line, provenance, or the outbox.

use std::collections::HashMap;

use crate::error::BrokerError;

/// An opaque credential capability for one service. Deliberately **not**
/// `Serialize`/`Clone`-into-audit and with a redacting `Debug`: the secret lives
/// only in memory and only the adapter that received the handle can read it.
pub struct CredentialHandle {
    service: String,
    secret: String,
}

impl CredentialHandle {
    pub fn service(&self) -> &str {
        &self.service
    }
    /// The secret material, for the adapter to present to its transport. Callers
    /// must never log or persist this — the type keeps it out of every audit path.
    pub fn expose_secret(&self) -> &str {
        &self.secret
    }
}

impl std::fmt::Debug for CredentialHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never render the secret — a Debug of a handle is safe to log.
        f.debug_struct("CredentialHandle")
            .field("service", &self.service)
            .field("secret", &"[REDACTED]")
            .finish()
    }
}

/// The credential store (O-24 cred). In-memory for the slice; a production Vault
/// would back this with the encrypted party store (F7) — the interface is what
/// matters here.
#[derive(Default)]
pub struct Vault {
    secrets: HashMap<String, String>,
}

impl Vault {
    pub fn new() -> Self {
        Vault::default()
    }

    /// Register a credential for a service.
    pub fn set(&mut self, service: impl Into<String>, secret: impl Into<String>) {
        self.secrets.insert(service.into(), secret.into());
    }

    /// Mint a handle for a service, or `MISSING_CREDENTIAL` if none is registered.
    pub fn handle(&self, service: &str) -> Result<CredentialHandle, BrokerError> {
        self.secrets
            .get(service)
            .map(|secret| CredentialHandle {
                service: service.to_string(),
                secret: secret.clone(),
            })
            .ok_or_else(|| BrokerError::MissingCredential(service.to_string()))
    }
}
