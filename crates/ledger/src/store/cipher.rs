//! At-rest encryption seam for `artifact` and `masterdata` (ADR-1).
//!
//! **S1 PLACEHOLDER — NOT REAL ENCRYPTION.** The brief scopes encryption to a
//! thin placeholder for S1 ("note it, don't gold-plate"). `IdentityCipher`
//! stores plaintext. The real design (ADR-1 §Artifact / §Masterdata) is
//! envelope encryption (installation KEK from OS keyring / passphrase-derived,
//! versioned DEKs, AEAD such as AES-256-GCM or XChaCha20-Poly1305). Keeping the
//! trait seam here means the fiscal/read/write paths do not change when real
//! encryption lands; only the `Cipher` impl does. Tracked as a deferral in
//! `spike-notes.md`.

pub(crate) trait Cipher {
    fn seal(&self, plaintext: &[u8]) -> Vec<u8>;
    /// Inverse of `seal`. Called by `ArtifactStore::get` (see the dead-code note
    /// there); no non-test caller in S1.
    #[allow(dead_code)]
    fn open(&self, ciphertext: &[u8]) -> Vec<u8>;
}

/// Placeholder: stores plaintext verbatim. Replace before any real data.
pub(crate) struct IdentityCipher;

impl Cipher for IdentityCipher {
    fn seal(&self, plaintext: &[u8]) -> Vec<u8> {
        plaintext.to_vec()
    }
    fn open(&self, ciphertext: &[u8]) -> Vec<u8> {
        ciphertext.to_vec()
    }
}
