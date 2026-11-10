//! The **consult disclosure projection** (`ConsultQuestion`) and the **consult
//! result** (`Advice`) — the two Domain-owned types on the `consult-llm` external
//! boundary (RF1, S5). This is the code side of `redaction-strategy-options.md` §4:
//! disclosure is a **default-deny allowlist projection built at source** (C3-typed),
//! not a denylist filter the Broker runs.
//!
//! ## Why these live in the Domain (not the Broker)
//! The Broker stays **schema-agnostic** (the M3 split). The party that knows what a
//! question actually needs disclosed is the caller/Domain, so the projection is built
//! here and handed to the Stream adapter *as the envelope body*. `Envelope::sent_hash`
//! then already covers exactly the disclosed bytes, and the S3 Preview-gate binding is
//! reused verbatim (bind-after-redact, Ba) — no new binding machinery.
//!
//! ## The type-level leak barrier (C3-typed)
//! A disclosure fact can only be built from a value that is [`DisclosureSafe`]. The
//! PII newtypes ([`aggountant_model::Iban`], [`aggountant_model::FinnishPersonId`]) do **not**
//! implement it, so **a value still typed as `Iban`/`FinnishPersonId` cannot be placed
//! into a disclosure fact** — that is a compile error, not a review miss. The barrier
//! stops the *accidental* leak (the safe outcome is the default). It is **not** a total
//! information-flow guarantee: a caller who deliberately `.expose()`s a PII newtype and
//! passes the resulting `String` has explicitly opted out, and free-text `topic`/
//! `question`/fact strings are trusted to the caller. (`DisclosureSafe` is a *sealed*
//! trait: downstream crates cannot add an `impl` for a PII type to smuggle it in.)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

mod sealed {
    /// Sealed so no downstream crate can widen the [`super::DisclosureSafe`] allowlist
    /// (e.g. add an impl for a PII newtype).
    pub trait Sealed {}
}

/// A value that is safe to place in an **external** disclosure: a plain descriptive
/// scalar with no PII semantics. Deliberately **not** implemented for
/// [`aggountant_model::Iban`] / [`aggountant_model::FinnishPersonId`], so those cannot enter
/// a [`ConsultQuestion`] — the leak is a compile error.
pub trait DisclosureSafe: sealed::Sealed {
    fn render(&self) -> String;
}

impl sealed::Sealed for String {}
impl DisclosureSafe for String {
    fn render(&self) -> String {
        self.clone()
    }
}
impl sealed::Sealed for &str {}
impl DisclosureSafe for &str {
    fn render(&self) -> String {
        (*self).to_string()
    }
}
impl sealed::Sealed for i64 {}
impl DisclosureSafe for i64 {
    fn render(&self) -> String {
        self.to_string()
    }
}
impl sealed::Sealed for rust_decimal::Decimal {}
impl DisclosureSafe for rust_decimal::Decimal {
    fn render(&self) -> String {
        self.to_string()
    }
}

/// One allowlisted fact in a disclosure: an explicit `key = value` the caller chose to
/// reveal. Constructed only from a [`DisclosureSafe`] value.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DisclosureFact {
    pub key: String,
    pub value: String,
}

impl DisclosureFact {
    /// Build a fact from an allowlisted value. The `value` bound is what makes a PII
    /// newtype a compile error here.
    pub fn new(key: impl Into<String>, value: impl DisclosureSafe) -> Self {
        DisclosureFact {
            key: key.into(),
            value: value.render(),
        }
    }
}

/// A **default-deny disclosure projection** for `consult-llm` (`CorePayload::ConsultQuestion`).
/// It contains *only* the facts the caller explicitly projected in — there is no fuller
/// body travelling alongside it, so nothing can leak by omission. The projection **is**
/// the Stream adapter's envelope body ([`ConsultQuestion::to_body`]).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ConsultQuestion {
    /// A coarse topic tag (e.g. `"vat-deductibility"`), not sensitive.
    pub topic: String,
    /// The explicit allowlist of disclosed facts.
    pub facts: Vec<DisclosureFact>,
    /// The natural-language question the model is asked.
    pub question: String,
}

impl ConsultQuestion {
    pub fn new(topic: impl Into<String>, question: impl Into<String>) -> Self {
        ConsultQuestion {
            topic: topic.into(),
            facts: Vec::new(),
            question: question.into(),
        }
    }

    /// Add one allowlisted fact (builder). Only a [`DisclosureSafe`] value is accepted.
    pub fn with_fact(mut self, key: impl Into<String>, value: impl DisclosureSafe) -> Self {
        self.facts.push(DisclosureFact::new(key, value));
        self
    }

    /// The canonical JSON body sent to `consult-llm` — exactly the projected bytes the
    /// `ApproveConsultDisclosure` gate binds via `sent_hash`.
    pub fn to_body(&self) -> Value {
        json!({
            "topic": self.topic,
            "facts": self.facts,
            "question": self.question,
        })
    }

    /// The set of top-level fact keys (for the positive/exhaustive allowlist test).
    pub fn fact_keys(&self) -> Vec<&str> {
        self.facts.iter().map(|f| f.key.as_str()).collect()
    }
}

/// A **local** source-lineage stamp (Bb): `SHA-256` of the raw pre-projection source
/// (the receipt/PII the projection was built from). It is written to the run/provenance
/// record so an auditor can later prove *which* internal payload a disclosure was
/// projected from — **without** the secret itself being retained. The hash never leaves
/// the machine; it is not part of the sent disclosure.
pub fn disclosure_source_hash(raw_source: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(raw_source);
    hex::encode(h.finalize())
}

/// The **consult result** — advice the agent may act on, but which is *not law*. Per
/// `candidate.md §4`, `Advice` **never reaches the ledger**: this type carries no
/// account, no posting, no `Money`, and there is deliberately **no** `From<Advice>` for
/// any ledger type and no method that yields a `VoucherDraft`/`Posting`. It only
/// *informs* a draft the human later approves and the Ledger validates + seals through
/// the ordinary S1 path. Its `citations` may become a [`crate::KnowledgeCandidate`]
/// gated by `ApproveKnowledgePR` — never a silent write.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Advice {
    pub summary: String,
    #[serde(default)]
    pub citations: Vec<String>,
}

impl Advice {
    pub fn new(summary: impl Into<String>, citations: Vec<String>) -> Self {
        Advice {
            summary: summary.into(),
            citations,
        }
    }

    /// Project this advice into a **knowledge candidate** — the review-gated write path
    /// (`ApproveKnowledgePR`). It carries only the summary + citations under a caller-
    /// chosen slug; nothing from the raw receipt or the disclosure source travels with
    /// it. The summary/citations are (untrusted) model output, so the candidate is
    /// *staged* and the human review gate — not construction — is what keeps a stray PII
    /// echo out of `knowledge`; it is never auto-cached (`candidate.md §2`).
    pub fn into_knowledge_candidate(self, slug: impl Into<String>) -> KnowledgeCandidate {
        KnowledgeCandidate {
            slug: slug.into(),
            summary: self.summary,
            citations: self.citations,
        }
    }
}

/// A staged, review-gated `knowledge` write (candidate.md §2). Its only fields are a
/// slug, a free-text summary, and citations — it carries no PII newtype and no ledger
/// figure. Note the summary/citations originate from the (untrusted) external model, so
/// this is **not** "PII-free by construction": the `ApproveKnowledgePR` human gate is the
/// control that a reviewer sees the exact bytes before they are committed (no auto-cache).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KnowledgeCandidate {
    pub slug: String,
    pub summary: String,
    pub citations: Vec<String>,
}

impl KnowledgeCandidate {
    /// Render the versioned free-text `knowledge` document (maturity scope for S5: a
    /// free-text markdown doc, no semantic search — `candidate.md §2`).
    pub fn to_document(&self) -> String {
        let mut doc = format!("# {}\n\n{}\n", self.slug, self.summary);
        if !self.citations.is_empty() {
            doc.push_str("\n## Citations\n\n");
            for c in &self.citations {
                doc.push_str(&format!("- {c}\n"));
            }
        }
        doc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_body_contains_only_allowlisted_keys() {
        let q = ConsultQuestion::new("vat-deductibility", "Is this VAT deductible?")
            .with_fact("expense_nature", "passenger-car leasing")
            .with_fact("counterparty_type", "car dealer");
        // The top-level body keys are exactly the fixed projection shape…
        let body = q.to_body();
        let mut keys: Vec<&str> = body
            .as_object()
            .unwrap()
            .keys()
            .map(|s| s.as_str())
            .collect();
        keys.sort();
        assert_eq!(keys, vec!["facts", "question", "topic"]);
        // …and the facts are exactly the two the caller projected in (nothing else).
        assert_eq!(q.fact_keys(), vec!["expense_nature", "counterparty_type"]);
    }

    #[test]
    fn source_lineage_hash_is_stable_and_source_dependent() {
        let a = disclosure_source_hash(b"raw receipt bytes with HETU 131052-308T");
        let b = disclosure_source_hash(b"raw receipt bytes with HETU 131052-308T");
        let c = disclosure_source_hash(b"a different source");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn disclosure_safe_renders_scalars() {
        // A compile-time allowlist: these types are DisclosureSafe. The PII newtypes
        // (Iban/FinnishPersonId) are NOT — putting one here would fail to compile, which
        // is the leak barrier (`tests/boundary.rs` guards that structurally too).
        let q = ConsultQuestion::new("t", "q")
            .with_fact("s", "text".to_string())
            .with_fact("n", 42i64);
        assert_eq!(q.facts[0].value, "text");
        assert_eq!(q.facts[1].value, "42");
    }
}
