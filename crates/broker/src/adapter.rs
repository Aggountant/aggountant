//! The adapter family (F11 traits + F12 transport seam).
//!
//! **Three interaction forms** (`candidate.md §6`): [`RequestResponse`] (the only
//! form with concrete adapters in v1), [`Subscription`] (inbound push — Holvi is
//! RR in v1, so no adapter ships; the `push-forgery-adr` guards the *first* one),
//! and [`Stream`] (`consult-llm`, S5). All three are declared so the contract is
//! expressible; only `RequestResponse` is exercised here.
//!
//! **The adapter-vs-Broker split.** An adapter owns *domain interpretation*: it
//! turns a raw authority response into an [`AdapterResponse`] or an
//! [`AdapterError`]. In particular, **the adapter — not the Broker — interprets a
//! `409 already_submitted` as [`AdapterResponse::RecoveredAlreadyProcessed`]**
//! (M3). The Broker never reads a status code; it only sees the adapter's verdict
//! and applies transport policy (retry / declare unknown / record).
//!
//! The concrete adapters talk to an injected [`Transport`] rather than a socket, so
//! there is no network in v1 and tests inject timeout / 409 / success through a
//! [`ScriptedTransport`]. The tested F12 code is the *interpretation*, which is
//! exactly the substrate the M3 gate probes.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::envelope::{DeliverySemantics, Envelope};
use crate::error::AdapterError;
use crate::vault::CredentialHandle;

/// The adapter's interpretation of a *successful-shaped* authority response — the
/// half of the split that carries domain meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterResponse {
    /// A fresh acceptance (e.g. `200`/`201`): the authority processed the request
    /// now. Carries the authority's receipt/reference and the parsed body.
    Accepted {
        receipt: String,
        body: serde_json::Value,
    },
    /// The authority reported the request was **already processed** — the adapter's
    /// interpretation of `409 already_submitted`. This is the M3 responsibility
    /// split made concrete: the mapping lives here, in the adapter, not the Broker.
    RecoveredAlreadyProcessed { receipt: String },
}

/// Whether a [`RequestResponse`] adapter performs a **read/pull** (idempotent —
/// safe to repeat, no double-effect hazard) or a **write/submit** (a filing / a
/// payment — at-most-once, outbox-recorded). The Broker enforces this before every
/// dispatch: `fetch`/`pull` only ever reach a [`AdapterKind::Pull`] adapter and
/// `submit` only a [`AdapterKind::Submit`] one. Without the guard a submit service
/// resolved through `fetch` would enter the *retrying* read path, and a
/// committed-then-timed-out write retried there is a double-submit — the exact
/// at-most-once breach this closes.
///
/// This is deliberately its **own** axis, not read off [`DeliverySemantics`]: the
/// kind is *what the operation does* (read vs write), whereas the semantics are the
/// *duplication risk* (`AtMostOnce`/`AtLeastOnce`). They correlate today (submits are
/// `AtMostOnce`, reads `AtLeastOnce`) but conflating them would let a future
/// `AtLeastOnce` submit or `AtMostOnce` read silently defeat the dispatch guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    /// A read/pull (`vero-verotili`, `email-pull`, `kauppatili-pull`): the Broker's
    /// `fetch`/`pull` path, which may retry freely because a read cannot double-fire.
    Pull,
    /// A write/submit (`synthetic-submit`, `pankki-maksu`, `tulorekisteri`): the
    /// Broker's `submit` path, with its idempotency/outbox/at-most-once discipline.
    Submit,
}

/// A synchronous request/response adapter (F11). Implemented by the three F12
/// adapters. `call` performs exactly one attempt; the Broker owns the retry loop.
pub trait RequestResponse {
    /// The service key this adapter answers to (matches [`Envelope::service`]).
    fn service(&self) -> &str;
    /// Whether this is a read/pull or a write/submit adapter. The Broker checks it
    /// before dispatch so a submit adapter can never enter the retrying read path
    /// (and a pull adapter can never reach `submit`).
    fn kind(&self) -> AdapterKind;
    /// The delivery semantics recorded in provenance and honored by the Broker
    /// (`AtMostOnce` submits are never auto-retried on `Unknown`).
    fn assurance(&self) -> DeliverySemantics;
    /// One attempt. A transport timeout MUST surface as [`AdapterError::Timeout`]
    /// (the Broker turns it into `Unknown`); a definitive rejection as
    /// [`AdapterError::Rejected`]; an already-processed report as
    /// [`AdapterResponse::RecoveredAlreadyProcessed`].
    fn call(
        &self,
        envelope: &Envelope,
        cred: &CredentialHandle,
    ) -> Result<AdapterResponse, AdapterError>;
}

/// Inbound push/webhook adapter (`candidate.md §6`). No adapter ships in v1 (the
/// first one triggers the `push-forgery-adr` signature-verification decision); the
/// trait exists so the three-form contract is complete.
pub trait Subscription {
    fn service(&self) -> &str;
    /// Verify + accept one pushed event. Forgery protection (signature check) is
    /// the `push-forgery-adr` obligation on the first real implementor.
    fn accept(&self, event: &serde_json::Value) -> Result<(), AdapterError>;
}

/// One chunk of a `consult-llm` stream turn (S5). This is the consult **wire
/// protocol**, owned by the adapter (like a status code) — *not* a domain schema: the
/// Broker never reads the `Final` body's meaning (that is `Advice`, a Domain concern).
/// The adapter maps a [`RawResponse`] to one of these; the Run interprets a `Final`
/// body into `Advice` and decides whether a `ToolCall` is executed locally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsultChunk {
    /// A partial completion (streamed text). Advisory; carries no binding content.
    Partial { text: String },
    /// The model asked to call a tool the Broker executes **locally** (`candidate.md
    /// §6`) — never a transport round trip. The Run runs it and re-projects + re-binds
    /// the tool result on the next outbound turn.
    ToolCall {
        tool: String,
        args: serde_json::Value,
    },
    /// The final turn: the opaque `Advice` body the Run parses.
    Final { body: serde_json::Value },
}

/// Streaming adapter (`consult-llm`, S5). Unlike [`RequestResponse`], a stream is
/// *turn-based*: each outbound turn sends a **projected** disclosure (or a tool result)
/// and returns the next [`ConsultChunk`]. The projection discipline applies to **every**
/// outbound turn (`redaction-strategy-options.md` §4 honest-cost #3), which is why each
/// turn is a fresh [`RawRequest`] the caller re-binds.
pub trait Stream {
    fn service(&self) -> &str;
    /// One outbound turn: send `request` and return the next chunk. `cred` authorizes;
    /// the transport is the dumb pipe. A rate limit surfaces as
    /// [`AdapterError::RateLimited`] so the Run can park on an `Effect::Wait`.
    fn turn(
        &self,
        request: &RawRequest,
        cred: &CredentialHandle,
    ) -> Result<ConsultChunk, AdapterError>;
}

// ---------------------------------------------------------------------------
// Transport seam (no network in v1; tests inject outcomes)
// ---------------------------------------------------------------------------

/// A raw request handed to a [`Transport`]. The adapter builds it from the
/// envelope + credential; the transport is a dumb pipe (no interpretation).
#[derive(Debug, Clone)]
pub struct RawRequest {
    pub service: String,
    pub operation: String,
    pub idempotency_key: String,
    pub body: serde_json::Value,
}

/// A raw authority response: a status code + a body, exactly what a real HTTP
/// client would hand back. The adapter (`interpret`) — not the Broker — maps it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawResponse {
    pub status: u16,
    pub body: serde_json::Value,
}

impl RawResponse {
    pub fn new(status: u16, body: serde_json::Value) -> Self {
        RawResponse { status, body }
    }
}

/// A transport-level failure — the *transport* has no notion of a domain response,
/// only whether bytes made a definitive round trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// No response arrived in time; the request may or may not have been processed.
    Timeout,
    /// The connection failed before the request was delivered (safe to retry).
    ConnectionRefused(String),
}

/// The pipe an adapter sends a [`RawRequest`] down. Production would wrap an HTTP
/// client; v1 ships no live transport, so this is the only seam a test needs.
pub trait Transport {
    fn round_trip(&self, req: &RawRequest) -> Result<RawResponse, TransportError>;
}

/// A deterministic transport that replays a queued script of outcomes, one per
/// call, and counts calls. The reusable test double for injecting timeout / 409 /
/// success into any adapter — shared by the broker's own tests and the `file-vat`
/// integration tests (the "fake/in-memory Vero-API adapter" the M3 brief asks for).
pub struct ScriptedTransport {
    script: Mutex<std::collections::VecDeque<Result<RawResponse, TransportError>>>,
    calls: Mutex<u32>,
}

impl ScriptedTransport {
    /// Build from a script; each round trip pops the next entry. Running past the
    /// end yields a timeout (a conservative "no answer" default).
    pub fn new(script: Vec<Result<RawResponse, TransportError>>) -> Self {
        ScriptedTransport {
            script: Mutex::new(script.into()),
            calls: Mutex::new(0),
        }
    }

    /// How many round trips have been performed — the assertion hook for
    /// "a timeout did NOT trigger a retry" (call count stays 1).
    pub fn call_count(&self) -> u32 {
        *self.calls.lock().unwrap()
    }
}

impl Transport for ScriptedTransport {
    fn round_trip(&self, _req: &RawRequest) -> Result<RawResponse, TransportError> {
        *self.calls.lock().unwrap() += 1;
        self.script
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Err(TransportError::Timeout))
    }
}
