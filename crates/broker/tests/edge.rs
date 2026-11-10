//! F11/F12/F9 edge-substrate tests — the M3 gate at the Broker level (the
//! `file-vat` Run reuses this substrate; here we prove the substrate itself).

use serde_json::json;

use aggountant_broker::contract::assert_request_response_contract;
use aggountant_broker::{
    classify_conflict_scripted, AdapterKind, AttemptOutcome, Broker, ConflictVerdict,
    DeliverySemantics, Envelope, PankkiMaksuAdapter, RawResponse, Registry, RemoteOutcome,
    RetryPolicy, ScriptedTransport, SyntheticSubmitAdapter, Transport, TransportError, Vault,
    VeroVerotiliAdapter, SVC_PANKKI_MAKSU, SVC_SYNTHETIC_SUBMIT, SVC_VERO_VEROTILI,
};

fn vault_for(service: &str) -> Vault {
    let mut v = Vault::new();
    v.set(service, "secret");
    v
}

fn submit_broker(home: &std::path::Path, transport: Box<dyn Transport>) -> Broker {
    let mut reg = Registry::new();
    reg.register(Box::new(SyntheticSubmitAdapter::new(transport)));
    Broker::new(home, vault_for(SVC_SYNTHETIC_SUBMIT), reg)
}

fn filing_envelope(nk: &str) -> Envelope {
    Envelope::new(
        SVC_SYNTHETIC_SUBMIT,
        "submit-vat-return",
        nk,
        json!({"period": "2026-Q1", "net_vat": "1500.00"}),
    )
}

// --- M3: timeout → Unknown, no blind retry ---------------------------------

#[test]
fn timeout_yields_unknown_without_a_blind_retry() {
    let home = tempfile::tempdir().unwrap();
    // A retry budget of 3 is available; the point is a *timeout* still exits after
    // one attempt (unlike a retryable transport failure, which would consume it).
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Err(TransportError::Timeout)])),
    )
    .with_retry(RetryPolicy { max_attempts: 3 });

    let outcome = broker
        .submit("agent", "file-2026-Q1", &filing_envelope("nk:1"))
        .unwrap();
    assert_eq!(outcome, RemoteOutcome::Unknown);

    // The outbox recorded exactly ONE attempt, outcome Unknown — proof there was no
    // blind retry (a retry would have appended a second attempt).
    let entry = broker
        .outbox_entry(SVC_SYNTHETIC_SUBMIT, "file-2026-Q1")
        .unwrap()
        .expect("an attempt was recorded");
    assert_eq!(entry.attempts.len(), 1, "a timeout must not be retried");
    // The parked Unknown carries its cause: a socket timeout, distinct from a proxy
    // 5xx or an unverifiable 409 (`broker-unknown-cause-observability`).
    assert!(
        matches!(&entry.attempts[0].outcome, AttemptOutcome::Unknown { code, .. } if code == "TIMEOUT"),
        "a timeout Unknown must record cause TIMEOUT, got {:?}",
        entry.attempts[0].outcome
    );
}

// --- M3: 409 already_submitted → the ADAPTER interprets it -----------------

#[test]
fn adapter_not_broker_interprets_409_as_recovered() {
    let home = tempfile::tempdir().unwrap();
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            409,
            // The authority confirms OUR idempotency key (== natural_key) as the
            // already-processed one — the identity the recovery is verified against.
            json!({"receipt": "already-filed-42", "idempotency_key": "nk:1"}),
        ))])),
    );
    let outcome = broker
        .submit("agent", "file-2026-Q1", &filing_envelope("nk:1"))
        .unwrap();
    assert_eq!(
        outcome,
        RemoteOutcome::RecoveredAlreadyProcessed {
            receipt: "already-filed-42".into()
        }
    );

    // Guard the responsibility split structurally: the Broker source must contain
    // no `409` literal — that interpretation lives in the adapter alone.
    let broker_src = include_str!("../src/broker.rs");
    assert!(
        !broker_src.contains("409"),
        "the Broker must not interpret status 409 — that is the adapter's job (M3)"
    );
}

// --- M3: the killer edge — timeout, then a deliberate retry meets the 409 ---

#[test]
fn unknown_then_driver_retry_recovers_via_remote_idempotency() {
    let home = tempfile::tempdir().unwrap();
    // First submit times out (Unknown); the request actually landed. A later,
    // driver-initiated retry meets the authority's 409 and recovers — NO double
    // filing, and the same sent_hash throughout.
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![
            Err(TransportError::Timeout),
            Ok(RawResponse::new(
                409,
                json!({"receipt": "R-recovered", "idempotency_key": "nk:1"}),
            )),
        ])),
    );
    let env = filing_envelope("nk:1");
    let first = broker.submit("agent", "file", &env).unwrap();
    assert_eq!(first, RemoteOutcome::Unknown);

    let second = broker.submit("agent", "file", &env).unwrap();
    assert_eq!(
        second,
        RemoteOutcome::RecoveredAlreadyProcessed {
            receipt: "R-recovered".into()
        }
    );

    // Two attempts, both with the SAME sent_hash (per-attempt provenance preserved).
    let entry = broker
        .outbox_entry(SVC_SYNTHETIC_SUBMIT, "file")
        .unwrap()
        .unwrap();
    assert_eq!(entry.attempts.len(), 2);
    assert_eq!(entry.attempts[0].sent_hash, entry.attempts[1].sent_hash);
    assert!(matches!(
        &entry.attempts[0].outcome,
        AttemptOutcome::Unknown { code, .. } if code == "TIMEOUT"
    ));
    assert!(matches!(
        entry.attempts[1].outcome,
        AttemptOutcome::RecoveredAlreadyProcessed { .. }
    ));
}

// --- M3: per-attempt sent_hash preserved across a retry --------------------

#[test]
fn retryable_transport_retries_with_a_stable_sent_hash() {
    let home = tempfile::tempdir().unwrap();
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![
            // A definitively-not-delivered transport failure (connection refused before
            // the bytes went out) is the only safe-to-retry case for a submit — every
            // 5xx is now ambiguous → Unknown, so it exits instead of retrying.
            Err(TransportError::ConnectionRefused("conn reset".into())),
            Ok(RawResponse::new(200, json!({"receipt": "R-ok"}))),
        ])),
    );
    let env = filing_envelope("nk:1");
    let expected_hash = env.sent_hash().unwrap();
    let outcome = broker.submit("agent", "file", &env).unwrap();
    assert_eq!(
        outcome,
        RemoteOutcome::Delivered {
            receipt: "R-ok".into()
        }
    );

    let entry = broker
        .outbox_entry(SVC_SYNTHETIC_SUBMIT, "file")
        .unwrap()
        .unwrap();
    assert_eq!(
        entry.attempts.len(),
        2,
        "one retry after the refused connection"
    );
    for a in &entry.attempts {
        assert_eq!(
            a.sent_hash, expected_hash,
            "sent_hash must be stable across attempts"
        );
    }
    // Attempt indices increment, provenance is per-attempt (not overwritten).
    assert_eq!(entry.attempts[0].provenance.attempt, 1);
    assert_eq!(entry.attempts[1].provenance.attempt, 2);
}

// --- M3: idempotency — a terminal success never fires twice -----------------

#[test]
fn terminal_success_is_replayed_not_resent() {
    let home = tempfile::tempdir().unwrap();
    // Script only ONE success; a second successful round trip would be a bug.
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            200,
            json!({"receipt": "R-1"}),
        ))])),
    );
    let env = filing_envelope("nk:1");
    let first = broker.submit("agent", "file", &env).unwrap();
    let second = broker.submit("agent", "file", &env).unwrap();
    assert_eq!(
        first, second,
        "the second submit replays, it does not re-send"
    );

    // Only one attempt was ever recorded — the replay short-circuited the transport.
    let entry = broker
        .outbox_entry(SVC_SYNTHETIC_SUBMIT, "file")
        .unwrap()
        .unwrap();
    assert_eq!(entry.attempts.len(), 1);
}

// --- M3 hardening: replay is bound to the effect identity -------------------

#[test]
fn replay_over_different_bytes_is_a_mismatch_not_a_false_success() {
    let home = tempfile::tempdir().unwrap();
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(
            vec![RawResponse::new(200, json!({"receipt": "R-1"}))]
                .into_iter()
                .map(Ok)
                .collect(),
        )),
    );
    // First submit succeeds and is recorded.
    broker
        .submit("agent", "file", &filing_envelope("nk:1"))
        .unwrap();
    // A second submit under the SAME effect_key but DIFFERENT bytes must NOT replay
    // the old success as if the new bytes were delivered.
    let tampered = Envelope::new(
        SVC_SYNTHETIC_SUBMIT,
        "submit-vat-return",
        "nk:1",
        json!({"period": "2026-Q1", "net_vat": "9999.00"}),
    );
    let err = broker.submit("agent", "file", &tampered).unwrap_err();
    assert_eq!(err.error_code(), "IDEMPOTENCY_MISMATCH");
}

// --- M3: ambiguous 5xx on an AtMostOnce submit is Unknown, not a retry ------

#[test]
fn ambiguous_500_on_submit_is_unknown_no_retry() {
    let home = tempfile::tempdir().unwrap();
    // A 500 could mean the authority committed the filing then failed to answer.
    // For a submit (AtMostOnce) that is Unknown — the Broker must not retry it.
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            500,
            json!({}),
        ))])),
    )
    .with_retry(RetryPolicy { max_attempts: 3 });
    let outcome = broker
        .submit("agent", "file", &filing_envelope("nk:1"))
        .unwrap();
    assert_eq!(outcome, RemoteOutcome::Unknown);
    let entry = broker
        .outbox_entry(SVC_SYNTHETIC_SUBMIT, "file")
        .unwrap()
        .unwrap();
    assert_eq!(
        entry.attempts.len(),
        1,
        "an ambiguous 500 must not be blind-retried"
    );
    // The Unknown records the ambiguous HTTP answer's cause (HTTP_500), distinct from a
    // socket timeout — an operator can tell them apart in the outbox.
    assert!(
        matches!(&entry.attempts[0].outcome, AttemptOutcome::Unknown { code, .. } if code == "HTTP_500"),
        "an ambiguous 500 Unknown must record cause HTTP_500, got {:?}",
        entry.attempts[0].outcome
    );
}

#[test]
fn service_unavailable_503_on_submit_is_unknown_no_retry() {
    let home = tempfile::tempdir().unwrap();
    // A `503` from a proxy in front of the authority may follow an upstream commit (the
    // proxy forwarded the request, the authority filed it, then the connection dropped).
    // For an AtMostOnce submit that is ambiguous → Unknown; the Broker must NOT retry it
    // (a blind retry is the double-filing hazard). Only a deliberate driver retry — which
    // reconciles via the authority's 409 — may follow.
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            503,
            json!({}),
        ))])),
    )
    .with_retry(RetryPolicy { max_attempts: 3 });
    let outcome = broker
        .submit("agent", "file", &filing_envelope("nk:1"))
        .unwrap();
    assert_eq!(outcome, RemoteOutcome::Unknown);
    let entry = broker
        .outbox_entry(SVC_SYNTHETIC_SUBMIT, "file")
        .unwrap()
        .unwrap();
    assert_eq!(
        entry.attempts.len(),
        1,
        "an ambiguous 503 on a submit must not be blind-retried"
    );
    // A 503 records HTTP_503 — distinct from a 500, a socket timeout, or an
    // unverifiable 409, so a triaging operator sees exactly which ambiguity parked it.
    assert!(
        matches!(&entry.attempts[0].outcome, AttemptOutcome::Unknown { code, .. } if code == "HTTP_503"),
        "an ambiguous 503 Unknown must record cause HTTP_503, got {:?}",
        entry.attempts[0].outcome
    );
}

// --- Regression: an ambiguous 503 on a PULL is still safely retryable -------

#[test]
fn service_unavailable_503_on_pull_is_retried() {
    let home = tempfile::tempdir().unwrap();
    // A read has no double-effect hazard, so a 503 (even proxy-sourced) is safe to retry:
    // the split is submit=Unknown, pull=retryable.
    let mut reg = Registry::new();
    reg.register(Box::new(VeroVerotiliAdapter::new(Box::new(
        ScriptedTransport::new(vec![
            Ok(RawResponse::new(503, json!({}))),
            Ok(RawResponse::new(200, json!({"balance": "0.00"}))),
        ]),
    ))));
    let broker = Broker::new(home.path(), vault_for(SVC_VERO_VEROTILI), reg)
        .with_retry(RetryPolicy { max_attempts: 3 });
    let env = Envelope::new(SVC_VERO_VEROTILI, "snapshot", "snap:2026-Q1", json!({}));
    let body = broker.fetch(&env).unwrap();
    assert_eq!(body, json!({"balance": "0.00"}));
}

// --- Regression: a 409 naming a DIFFERENT key is a definitive rejection ------

#[test]
fn different_key_409_is_a_definitive_rejection() {
    let home = tempfile::tempdir().unwrap();
    // The authority returns a 409 that confirms a DIFFERENT key — the conflict is with
    // some other resource, so our request was provably NOT the already-processed one.
    // That is a genuine, terminal rejection (not a false recovery).
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            409,
            json!({"receipt": "SOMEONE-ELSE", "idempotency_key": "nk:other"}),
        ))])),
    );
    let outcome = broker
        .submit("agent", "file", &filing_envelope("nk:1"))
        .unwrap();
    match outcome {
        RemoteOutcome::Rejected { code, .. } => assert_eq!(code, "CONFLICT_OTHER_KEY"),
        other => panic!("a different-key 409 must be Rejected, got {other:?}"),
    }
}

// --- Regression: an UNVERIFIABLE 409 is ambiguous → Unknown, NOT terminal ----

#[test]
fn unverifiable_409_is_unknown_not_a_false_terminal() {
    let home = tempfile::tempdir().unwrap();
    // A 409 with NO identity echoed cannot be told apart from "your prior attempt
    // already landed". Reporting a terminal Rejected for a possibly-committed filing
    // would be the mirror-image of laundering a false success — a caller could then
    // resubmit under a fresh key and double-file. It must surface as Unknown (no retry),
    // to be reconciled by a deliberate retry that meets a *verified* 409.
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            409,
            json!({"error": "conflict"}),
        ))])),
    )
    .with_retry(RetryPolicy { max_attempts: 3 });
    let outcome = broker
        .submit("agent", "file", &filing_envelope("nk:1"))
        .unwrap();
    assert_eq!(outcome, RemoteOutcome::Unknown);
    let entry = broker
        .outbox_entry(SVC_SYNTHETIC_SUBMIT, "file")
        .unwrap()
        .unwrap();
    assert_eq!(
        entry.attempts.len(),
        1,
        "an unverifiable 409 must not be blind-retried"
    );
    // The Unknown records HTTP_409 — an ambiguous *HTTP answer*, distinct from a
    // no-answer socket timeout, so the outbox trail tells the two apart.
    assert!(
        matches!(&entry.attempts[0].outcome, AttemptOutcome::Unknown { code, .. } if code == "HTTP_409"),
        "an unverifiable 409 Unknown must record cause HTTP_409, got {:?}",
        entry.attempts[0].outcome
    );
}

// --- Regression: each submit adapter owns its OWN 409 conflict-body schema ---

/// A live-shaped Vero classifier: this authority echoes the confirmed key *nested*
/// under `error.idempotency_key`, not as a top-level field. It owns its own body
/// schema — the whole point of a per-adapter [`aggountant_broker::ConflictClassifier`].
fn nested_vero_classifier(body: &serde_json::Value, key: &str) -> ConflictVerdict {
    match body
        .pointer("/error/idempotency_key")
        .and_then(|v| v.as_str())
    {
        Some(k) if k == key => ConflictVerdict::OurKey,
        Some(_) => ConflictVerdict::OtherKey,
        None => ConflictVerdict::Unverifiable,
    }
}

#[test]
fn per_adapter_classifier_parses_this_authoritys_own_409_schema() {
    // A 409 body in THIS authority's schema: the confirmed key is nested under
    // `error.idempotency_key` (not top-level, as the default scripted schema expects).
    let nested_body = json!({"receipt": "filed-9", "error": {"idempotency_key": "nk:1"}});

    // With the authority's own classifier the nested key is read → an idempotent
    // recovery, exactly as a top-level echo would be under the default schema.
    let home = tempfile::tempdir().unwrap();
    let mut reg = Registry::new();
    reg.register(Box::new(SyntheticSubmitAdapter::with_classifier(
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            409,
            nested_body.clone(),
        ))])),
        nested_vero_classifier,
    )));
    let broker = Broker::new(home.path(), vault_for(SVC_SYNTHETIC_SUBMIT), reg);
    let outcome = broker
        .submit("agent", "file", &filing_envelope("nk:1"))
        .unwrap();
    assert_eq!(
        outcome,
        RemoteOutcome::RecoveredAlreadyProcessed {
            receipt: "filed-9".into()
        },
        "the per-adapter classifier must read this authority's nested key echo"
    );

    // The SAME nested body through the DEFAULT (scripted-schema) adapter finds no
    // top-level key → Unverifiable → Unknown. Proof the classifier is genuinely
    // per-adapter, not one shared hardcoded parser: swapping it changes the verdict.
    let home2 = tempfile::tempdir().unwrap();
    let broker_default = submit_broker(
        home2.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            409,
            nested_body,
        ))])),
    );
    let outcome_default = broker_default
        .submit("agent", "file", &filing_envelope("nk:1"))
        .unwrap();
    assert_eq!(
        outcome_default,
        RemoteOutcome::Unknown,
        "the default top-level parser cannot see a nested key → ambiguous, not a recovery"
    );
}

/// Build a submit broker whose Vero adapter uses a custom [per-adapter classifier].
fn submit_broker_with_classifier(
    home: &std::path::Path,
    transport: Box<dyn Transport>,
    classify: fn(&serde_json::Value, &str) -> ConflictVerdict,
) -> Broker {
    let mut reg = Registry::new();
    reg.register(Box::new(SyntheticSubmitAdapter::with_classifier(
        transport, classify,
    )));
    Broker::new(home, vault_for(SVC_SYNTHETIC_SUBMIT), reg)
}

#[test]
fn per_adapter_classifier_other_and_unverifiable_map_through_interpret() {
    // The load-bearing architectural claim: only body *parsing* moved to the adapter;
    // the ternary safety *mapping* stays in `interpret`. Prove it holds for a CUSTOM
    // classifier's OtherKey and Unverifiable verdicts (not just OurKey), so a future
    // regression in interpret's dispatch on a custom classifier is caught.

    // OtherKey (nested key names a DIFFERENT request) → definitive Rejected.
    let home = tempfile::tempdir().unwrap();
    let broker = submit_broker_with_classifier(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            409,
            json!({"error": {"idempotency_key": "nk:other"}}),
        ))])),
        nested_vero_classifier,
    );
    match broker
        .submit("agent", "file", &filing_envelope("nk:1"))
        .unwrap()
    {
        RemoteOutcome::Rejected { code, .. } => assert_eq!(code, "CONFLICT_OTHER_KEY"),
        other => panic!("a custom-classifier OtherKey must be Rejected, got {other:?}"),
    }

    // Unverifiable (nested key absent) → ambiguous Unknown, never a terminal state.
    let home2 = tempfile::tempdir().unwrap();
    let broker2 = submit_broker_with_classifier(
        home2.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            409,
            json!({"error": {}}),
        ))])),
        nested_vero_classifier,
    );
    assert_eq!(
        broker2
            .submit("agent", "file", &filing_envelope("nk:1"))
            .unwrap(),
        RemoteOutcome::Unknown,
        "a custom-classifier Unverifiable must be Unknown, not a terminal state"
    );
}

// --- Regression: the default classifier treats malformed evidence as Unverifiable ---

#[test]
fn scripted_classifier_treats_present_non_string_field_as_unverifiable() {
    // A present-but-non-string identity field is MALFORMED evidence, not absent
    // evidence. It must not be silently dropped: a lone matching sibling would then
    // launder a contradictory body into a false OurKey (a false terminal recovery).
    for malformed in [
        json!({"idempotency_key": "nk:1", "natural_key": 123}),
        json!({"idempotency_key": "nk:1", "natural_key": null}),
        json!({"idempotency_key": "nk:1", "natural_key": ["nk:1"]}),
        json!({"idempotency_key": "nk:1", "natural_key": {"k": "nk:1"}}),
        // The sole identity field is itself non-string — nothing verifiable at all.
        json!({"idempotency_key": 1}),
    ] {
        assert_eq!(
            classify_conflict_scripted(&malformed, "nk:1"),
            ConflictVerdict::Unverifiable,
            "a present non-string identity field must be Unverifiable, not a false OurKey: {malformed}"
        );
    }

    // Sanity: the happy paths still classify as before.
    assert_eq!(
        classify_conflict_scripted(&json!({"idempotency_key": "nk:1"}), "nk:1"),
        ConflictVerdict::OurKey
    );
    assert_eq!(
        classify_conflict_scripted(&json!({"idempotency_key": "nk:other"}), "nk:1"),
        ConflictVerdict::OtherKey
    );
    // Two string fields disagreeing (one ours, one other) → contradictory → Unverifiable.
    assert_eq!(
        classify_conflict_scripted(
            &json!({"idempotency_key": "nk:1", "natural_key": "nk:other"}),
            "nk:1"
        ),
        ConflictVerdict::Unverifiable
    );

    // End-to-end: a malformed-field 409 must reach the Broker as Unknown, never a
    // terminal RecoveredAlreadyProcessed.
    let home = tempfile::tempdir().unwrap();
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            409,
            json!({"receipt": "x", "idempotency_key": "nk:1", "natural_key": 123}),
        ))])),
    );
    assert_eq!(
        broker
            .submit("agent", "file", &filing_envelope("nk:1"))
            .unwrap(),
        RemoteOutcome::Unknown,
        "a malformed-field 409 must not launder into a terminal recovery"
    );
}

// --- Regression: an uncommon 5xx (not one of 500/502/503/504) is still Unknown

#[test]
fn uncommon_5xx_on_submit_is_unknown() {
    let home = tempfile::tempdir().unwrap();
    // 507/599/520 etc. are just as ambiguous as 500 for an AtMostOnce submit — the
    // whole 500..=599 class must map to Unknown, not fall through to a terminal error.
    for status in [507u16, 520, 599] {
        let broker = submit_broker(
            home.path(),
            Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
                status,
                json!({}),
            ))])),
        );
        let effect = format!("file-{status}");
        let outcome = broker
            .submit("agent", &effect, &filing_envelope("nk:1"))
            .unwrap();
        assert_eq!(
            outcome,
            RemoteOutcome::Unknown,
            "HTTP {status} must be Unknown"
        );
        // ...and the whole-range mapping records the specific HTTP_<status> cause, so an
        // uncommon proxy 5xx is still distinguishable in the outbox (not flattened).
        let entry = broker
            .outbox_entry(SVC_SYNTHETIC_SUBMIT, &effect)
            .unwrap()
            .unwrap();
        assert_eq!(
            unknown_code(&entry.attempts[0].outcome),
            format!("HTTP_{status}"),
            "HTTP {status} must record its own cause code"
        );
    }
}

// --- broker-unknown-cause-observability: parked Unknowns carry their cause ---

/// The cause `code` of an [`AttemptOutcome::Unknown`], or a panic if the outcome is
/// something else — the whole issue is that a parked Unknown must expose *why*.
fn unknown_code(outcome: &AttemptOutcome) -> &str {
    match outcome {
        AttemptOutcome::Unknown { code, .. } => code,
        other => panic!("expected an Unknown outcome, got {other:?}"),
    }
}

#[test]
fn distinct_ambiguous_causes_are_distinguishable_in_the_outbox() {
    // The heart of `broker-unknown-cause-observability`: three different ambiguities —
    // a socket timeout, a proxy 5xx, and an unverifiable 409 — all yield the SAME
    // caller-facing `RemoteOutcome::Unknown`, but an operator triaging the parked
    // outbox entries must be able to tell them apart. Each records a distinct cause.
    let home = tempfile::tempdir().unwrap();
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![
            Err(TransportError::Timeout),
            Ok(RawResponse::new(503, json!({}))),
            Ok(RawResponse::new(409, json!({"error": "conflict"}))),
        ])),
    );

    // Three independent effects, each hitting one of the three ambiguous causes.
    for effect in ["file-timeout", "file-503", "file-409"] {
        assert_eq!(
            broker
                .submit("agent", effect, &filing_envelope("nk:1"))
                .unwrap(),
            RemoteOutcome::Unknown,
            "every ambiguous cause is the same caller-facing Unknown"
        );
    }

    let code_for = |effect: &str| -> String {
        let entry = broker
            .outbox_entry(SVC_SYNTHETIC_SUBMIT, effect)
            .unwrap()
            .unwrap();
        unknown_code(&entry.attempts[0].outcome).to_string()
    };

    // ...but the durable trail distinguishes them — the observability the unit variant
    // used to discard.
    assert_eq!(code_for("file-timeout"), "TIMEOUT");
    assert_eq!(code_for("file-503"), "HTTP_503");
    assert_eq!(code_for("file-409"), "HTTP_409");

    // And all three are genuinely different, not accidentally collapsed.
    let codes = [
        code_for("file-timeout"),
        code_for("file-503"),
        code_for("file-409"),
    ];
    let unique: std::collections::HashSet<&String> = codes.iter().collect();
    assert_eq!(
        unique.len(),
        3,
        "the three ambiguous causes must stay distinct"
    );
}

#[test]
fn an_ambiguous_causes_detail_is_preserved_not_discarded() {
    // The `detail` string the old unit variant threw away is now carried through, so a
    // human reading the outbox sees the adapter's own elaboration, not just a code.
    let home = tempfile::tempdir().unwrap();
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            502,
            json!({}),
        ))])),
    );
    broker
        .submit("agent", "file", &filing_envelope("nk:1"))
        .unwrap();
    let entry = broker
        .outbox_entry(SVC_SYNTHETIC_SUBMIT, "file")
        .unwrap()
        .unwrap();
    match &entry.attempts[0].outcome {
        AttemptOutcome::Unknown { code, detail } => {
            assert_eq!(code, "HTTP_502");
            assert!(
                detail.contains("502") && detail.contains("may have been processed"),
                "the ambiguous cause detail must survive into the trail, got {detail:?}"
            );
        }
        other => panic!("expected Unknown, got {other:?}"),
    }
}

// --- Contract harness across all three F12 adapters ------------------------

#[test]
fn all_three_adapters_satisfy_the_request_response_contract() {
    assert_request_response_contract(
        |t| Box::new(SyntheticSubmitAdapter::new(t)),
        SVC_SYNTHETIC_SUBMIT,
        AdapterKind::Submit,
        DeliverySemantics::AtMostOnce,
    );
    assert_request_response_contract(
        |t| Box::new(PankkiMaksuAdapter::new(t)),
        SVC_PANKKI_MAKSU,
        AdapterKind::Submit,
        DeliverySemantics::AtMostOnce,
    );
    assert_request_response_contract(
        |t| Box::new(VeroVerotiliAdapter::new(t)),
        SVC_VERO_VEROTILI,
        AdapterKind::Pull,
        DeliverySemantics::AtLeastOnce,
    );
}

// --- Payment net-invariant demonstrated with a RequestResponse example ------

#[test]
fn pankki_maksu_is_a_request_response_example() {
    // The net-invariant itself is enforced in the domain/process before the payment
    // envelope is built; here we show the payment travels the same RR substrate.
    let home = tempfile::tempdir().unwrap();
    let mut reg = Registry::new();
    reg.register(Box::new(PankkiMaksuAdapter::new(Box::new(
        ScriptedTransport::new(vec![Ok(RawResponse::new(200, json!({"receipt": "PMT-1"})))]),
    ))));
    let broker = Broker::new(home.path(), vault_for(SVC_PANKKI_MAKSU), reg);
    let env = Envelope::new(
        SVC_PANKKI_MAKSU,
        "pay",
        "pay:2026-Q1",
        json!({"amount": "1300.00", "iban": "FI21..."}),
    );
    let outcome = broker.submit("agent", "pay-2026-Q1", &env).unwrap();
    assert_eq!(
        outcome,
        RemoteOutcome::Delivered {
            receipt: "PMT-1".into()
        }
    );
}

// --- S4: the Tulorekisteri adapter recovers a duplicate report (409) --------

#[test]
fn tulorekisteri_409_is_recovered_not_double_reported() {
    use aggountant_broker::{TulorekisteriAdapter, SVC_TULOREKISTERI};

    let home = tempfile::tempdir().unwrap();
    // First attempt times out (Unknown), the deliberate retry meets a 409 the
    // adapter interprets as already-processed — so a report is never filed twice.
    let transport: Box<dyn Transport> = Box::new(ScriptedTransport::new(vec![
        Err(TransportError::Timeout),
        Ok(RawResponse::new(
            409,
            json!({"receipt": "IR-DUP", "idempotency_key": "report:emp-1:2026-02-28"}),
        )),
    ]));
    let mut reg = Registry::new();
    reg.register(Box::new(TulorekisteriAdapter::new(transport)));
    let broker = Broker::new(home.path(), vault_for(SVC_TULOREKISTERI), reg);

    let env = Envelope::new(
        SVC_TULOREKISTERI,
        "palkkatietoilmoitus",
        "report:emp-1:2026-02-28",
        json!({"payment_date": "2026-02-28", "gross": "3000.00"}),
    );
    assert_eq!(
        broker.submit("payroll", "report:emp-1", &env).unwrap(),
        RemoteOutcome::Unknown
    );
    // Deliberate retry recovers the duplicate.
    assert_eq!(
        broker.submit("payroll", "report:emp-1", &env).unwrap(),
        RemoteOutcome::RecoveredAlreadyProcessed {
            receipt: "IR-DUP".into()
        }
    );
}
