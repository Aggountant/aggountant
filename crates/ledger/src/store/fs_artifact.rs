//! Content-addressed (`sha256`) filesystem blob store (F8 / ADR-1 §Artifact).
//! Sharded paths, atomic publish (temp → fsync → rename → dir fsync), and a
//! placeholder cipher at rest (see `cipher.rs` — real envelope encryption is
//! deferred per the S1 brief). The catalog is the filesystem itself in S1; the
//! ADR's SQLite catalog (retention/ref-count/key-version) is a later concern.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest, Sha256};

use aggountant_model::Hash;

use crate::error::LedgerError;
use crate::store::cipher::{Cipher, IdentityCipher};
use crate::store::ArtifactStore;

pub(crate) struct FsArtifactStore {
    root: PathBuf,
    cipher: IdentityCipher,
}

fn sha256(bytes: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    Hash::from_bytes(out)
}

impl FsArtifactStore {
    pub(crate) fn open(root: &Path) -> Result<Self, LedgerError> {
        fs::create_dir_all(root).map_err(|e| LedgerError::Store(e.to_string()))?;
        Ok(FsArtifactStore {
            root: root.to_path_buf(),
            cipher: IdentityCipher,
        })
    }

    fn blob_path(&self, addr: &Hash) -> PathBuf {
        let hex = addr.to_hex();
        let (shard, rest) = hex.split_at(2);
        self.root.join(shard).join(format!("{rest}.blob"))
    }
}

impl ArtifactStore for FsArtifactStore {
    fn put(&self, plaintext: &[u8]) -> Result<Hash, LedgerError> {
        let addr = sha256(plaintext);
        let path = self.blob_path(&addr);
        if path.exists() {
            return Ok(addr); // content-addressed dedup
        }
        let dir = path.parent().expect("blob path has a parent");
        fs::create_dir_all(dir).map_err(|e| LedgerError::Store(e.to_string()))?;

        let ciphertext = self.cipher.seal(plaintext);
        // Unique temp name per attempt (pid + process-local counter) so two
        // concurrent `put`s of the same content never truncate each other's
        // in-flight temp file. `create_new` refuses to reuse an existing name.
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp = dir.join(format!(
            ".{}.{}.{}.tmp",
            addr.to_hex(),
            std::process::id(),
            unique
        ));
        {
            let mut f = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&tmp)
                .map_err(|e| LedgerError::Store(e.to_string()))?;
            f.write_all(&ciphertext)
                .map_err(|e| LedgerError::Store(e.to_string()))?;
            f.sync_all()
                .map_err(|e| LedgerError::Store(e.to_string()))?;
        }
        fs::rename(&tmp, &path).map_err(|e| LedgerError::Store(e.to_string()))?;
        // Best-effort directory durability so the rename survives a crash.
        if let Ok(dirf) = File::open(dir) {
            let _ = dirf.sync_all();
        }
        Ok(addr)
    }

    fn exists(&self, addr: &Hash) -> Result<bool, LedgerError> {
        Ok(self.blob_path(addr).exists())
    }

    fn get(&self, addr: &Hash) -> Result<Vec<u8>, LedgerError> {
        let path = self.blob_path(addr);
        let ciphertext =
            fs::read(&path).map_err(|_| LedgerError::ArtifactNotFound(addr.to_hex()))?;
        let plaintext = self.cipher.open(&ciphertext);
        // Content-address integrity: what we return must hash to its address.
        if sha256(&plaintext) != *addr {
            return Err(LedgerError::Store(format!(
                "artifact {} failed content-address check",
                addr.to_hex()
            )));
        }
        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_get_exists_and_dedup() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsArtifactStore::open(dir.path()).unwrap();
        let bytes = b"receipt bytes";

        let addr = store.put(bytes).unwrap();
        assert!(store.exists(&addr).unwrap());
        assert_eq!(store.get(&addr).unwrap(), bytes);

        // Content-addressed: storing the same bytes yields the same address.
        assert_eq!(store.put(bytes).unwrap(), addr);

        // A never-stored address is absent.
        assert!(!store.exists(&Hash::from_bytes([0x22; 32])).unwrap());
    }
}
