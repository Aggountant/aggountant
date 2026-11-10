//! Adapter-kind enforcement (`broker-enforce-adapter-kind`). The Broker resolves
//! `fetch`/`pull` and `submit` adapters from the SAME request/response registry, so
//! without a guard a submit service (`synthetic-submit`) could be invoked through the
//! *retrying* `fetch` path — and a committed-then-timed-out write retried there is a
//! double-submit, bypassing the whole at-most-once guarantee. Symmetrically a pull
//! adapter handed to `submit` would take the outbox/idempotency path it was never
//! meant for. The Broker now checks [`AdapterKind`] before dispatch and refuses the
//! mismatch, *before* any transport round trip or outbox write.

use serde_json::json;

use aggountant_broker::{
    AdapterKind, Broker, BrokerError, EmailPullAdapter, Envelope, PankkiMaksuAdapter, PullOutcome,
    RawRequest, RawResponse, Registry, RetryPolicy, ScriptedTransport, SyntheticSubmitAdapter,
    Transport, TransportError, Vault, VeroVerotiliAdapter, SVC_EMAIL_PULL, SVC_PANKKI_MAKSU,
    SVC_SYNTHETIC_SUBMIT, SVC_VERO_VEROTILI,
};

fn vault_for(service: &str) -> Vault {
    let mut v = Vault::new();
    v.set(service, "secret");
    v
}

/// A transport that panics if it is ever asked to round-trip. Registering an adapter
/// over it turns "the dispatch guard fired *before* any network call" into a hard,
/// unmissable assertion: reaching dispatch would panic the test instead of returning
/// a clean `AdapterKindMismatch`.
struct NeverTransport;
impl Transport for NeverTransport {
    fn round_trip(&self, _req: &RawRequest) -> Result<RawResponse, TransportError> {
        panic!("the kind guard must refuse the operation BEFORE any transport dispatch");
    }
}

// --- The core hazard: a submit adapter must not enter the retrying fetch path ---

#[test]
fn fetch_on_a_submit_adapter_is_refused_before_dispatch() {
    let home = tempfile::tempdir().unwrap();
    let mut reg = Registry::new();
    // A submit adapter (AtMostOnce) registered under its own service key.
    reg.register(Box::new(SyntheticSubmitAdapter::new(Box::new(
        NeverTransport,
    ))));
    // A generous retry budget — the point is the guard fires before the loop, not
    // that the loop happens to exit.
    let broker = Broker::new(home.path(), vault_for(SVC_SYNTHETIC_SUBMIT), reg)
        .with_retry(RetryPolicy { max_attempts: 3 });

    let env = Envelope::new(SVC_SYNTHETIC_SUBMIT, "submit-vat-return", "nk:1", json!({}));
    let err = broker.fetch(&env).unwrap_err();
    // The structured error names the service and both kinds (expected Pull, actual
    // Submit) so a caller can inspect the mismatch, not just its code.
    assert!(
        matches!(
            &err,
            BrokerError::AdapterKindMismatch { service, expected, actual }
                if service == SVC_SYNTHETIC_SUBMIT
                    && *expected == AdapterKind::Pull
                    && *actual == AdapterKind::Submit
        ),
        "fetch() must refuse a submit adapter with a precise kind mismatch, got {err:?} \
         (a committed-then-timed-out write retried on the read path is a double-submit)"
    );
}

#[test]
fn pull_on_a_submit_adapter_is_refused_before_dispatch() {
    let home = tempfile::tempdir().unwrap();
    let mut reg = Registry::new();
    reg.register(Box::new(PankkiMaksuAdapter::new(Box::new(NeverTransport))));
    let broker = Broker::new(home.path(), vault_for(SVC_PANKKI_MAKSU), reg);

    let env = Envelope::new(SVC_PANKKI_MAKSU, "pay", "pay:1", json!({}));
    let err = broker.pull(&env).unwrap_err();
    assert_eq!(err.error_code(), "ADAPTER_KIND_MISMATCH");
}

// --- The mirror image: a pull/read adapter must not reach the submit path -------

#[test]
fn submit_on_a_pull_adapter_is_refused_and_writes_no_outbox() {
    let home = tempfile::tempdir().unwrap();
    let mut reg = Registry::new();
    // A read adapter (AtLeastOnce) — never meant for the idempotency/outbox path.
    reg.register(Box::new(VeroVerotiliAdapter::new(Box::new(NeverTransport))));
    let broker = Broker::new(home.path(), vault_for(SVC_VERO_VEROTILI), reg);

    let env = Envelope::new(SVC_VERO_VEROTILI, "snapshot", "snap:1", json!({}));
    let err = broker.submit("agent", "eff-1", &env).unwrap_err();
    assert!(
        matches!(
            &err,
            BrokerError::AdapterKindMismatch { expected, actual, .. }
                if *expected == AdapterKind::Submit && *actual == AdapterKind::Pull
        ),
        "submit() must refuse a pull adapter, got {err:?}"
    );

    // Crucially, the refusal happened before ANY outbox write — no `in_flight` row
    // exists for an effect that was never dispatched.
    let entry = broker.outbox_entry(SVC_VERO_VEROTILI, "eff-1").unwrap();
    assert!(
        entry.is_none(),
        "a kind-refused submit must not leave an outbox attempt row"
    );
}

#[test]
fn kind_mismatch_is_refused_before_credential_lookup() {
    // The guard must be the FIRST gate: a mismatched kind wins even when the Vault
    // holds no credential for the service. This locks the ordering — resolve_kind()
    // runs before the vault/outbox work — so the guard can never be reordered behind
    // a step that has already touched durable/credential state.
    let home = tempfile::tempdir().unwrap();
    let mut reg = Registry::new();
    reg.register(Box::new(SyntheticSubmitAdapter::new(Box::new(
        NeverTransport,
    ))));
    // An EMPTY vault — no credential for SVC_SYNTHETIC_SUBMIT.
    let broker = Broker::new(home.path(), Vault::new(), reg);

    let env = Envelope::new(SVC_SYNTHETIC_SUBMIT, "submit-vat-return", "nk:1", json!({}));
    let err = broker.fetch(&env).unwrap_err();
    assert_eq!(
        err.error_code(),
        "ADAPTER_KIND_MISMATCH",
        "kind mismatch must win over a missing credential, got {err:?}"
    );
}

// --- Positive control: the legitimate pairings are unaffected by the guard -------

#[test]
fn pull_on_a_pull_adapter_still_works() {
    let home = tempfile::tempdir().unwrap();
    let mut reg = Registry::new();
    reg.register(Box::new(EmailPullAdapter::new(Box::new(
        ScriptedTransport::new(vec![Ok(RawResponse::new(
            200,
            json!({"doc": "receipt.pdf"}),
        ))]),
    ))));
    let broker = Broker::new(home.path(), vault_for(SVC_EMAIL_PULL), reg);

    let env = Envelope::new(SVC_EMAIL_PULL, "pull", "txn:1", json!({}));
    assert_eq!(
        broker.pull(&env).unwrap(),
        PullOutcome::Found(json!({"doc": "receipt.pdf"})),
        "a Pull adapter on the pull path must still be accepted by the guard"
    );
}

#[test]
fn submit_on_a_submit_adapter_still_works() {
    let home = tempfile::tempdir().unwrap();
    let mut reg = Registry::new();
    reg.register(Box::new(SyntheticSubmitAdapter::new(Box::new(
        ScriptedTransport::new(vec![Ok(RawResponse::new(200, json!({"receipt": "R-ok"})))]),
    ))));
    let broker = Broker::new(home.path(), vault_for(SVC_SYNTHETIC_SUBMIT), reg);

    let env = Envelope::new(SVC_SYNTHETIC_SUBMIT, "submit-vat-return", "nk:1", json!({}));
    // A Submit adapter on the submit path is accepted (delivered), proving the guard
    // only refuses the mismatch, never the legitimate pairing.
    let outcome = broker.submit("agent", "eff-1", &env).unwrap();
    assert_eq!(outcome.receipt(), Some("R-ok"));
}
