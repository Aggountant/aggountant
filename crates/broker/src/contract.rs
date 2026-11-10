//! The adapter **contract-test harness** (F11). One reusable set of assertions
//! every [`RequestResponse`] adapter must satisfy, so a new F12 adapter is proven
//! against the family contract rather than ad-hoc. It nails the invariants the
//! Broker relies on:
//! - a `200` maps to [`AdapterResponse::Accepted`];
//! - a transport **timeout** maps to [`AdapterError::Timeout`] (never a domain
//!   outcome) — the Broker's `Unknown` depends on this;
//! - a `5xx` on a pull is a *retryable* transport error; on an `AtMostOnce` submit it
//!   is ambiguous → [`AdapterError::AmbiguousDelivery`] (the double-effect guard),
//!   never a blind retry;
//! - a `409` is recovered **iff** the adapter declares recovery semantics **and** the
//!   response confirms our idempotency key; a `409` naming a *different* key is a
//!   definitive rejection, and an *unverifiable* `409` on a submit is ambiguous →
//!   [`AdapterError::AmbiguousDelivery`]/`Unknown` (never a false already-processed
//!   success, never a false terminal rejection) — the adapter-vs-Broker split,
//!   verified at the adapter boundary. Both ambiguous causes are `AmbiguousDelivery`,
//!   kept distinct from a *no-answer* [`AdapterError::Timeout`]
//!   (`broker-unknown-cause-observability`).

use crate::adapter::{
    AdapterKind, AdapterResponse, RawResponse, RequestResponse, ScriptedTransport, Transport,
    TransportError,
};
use crate::envelope::{DeliverySemantics, Envelope};
use crate::error::AdapterError;
use crate::vault::Vault;
use serde_json::json;

/// Build an adapter from a transport (the shape of every F12 constructor).
pub type AdapterCtor = fn(Box<dyn Transport>) -> Box<dyn RequestResponse>;

fn cred(service: &str) -> crate::vault::CredentialHandle {
    let mut v = Vault::new();
    v.set(service, "test-secret");
    v.handle(service).unwrap()
}

fn env(service: &str) -> Envelope {
    Envelope::new(service, "op", "nk-contract", json!({"x": 1}))
}

/// Run the full RequestResponse contract against `ctor`. `service` is the expected
/// service key; `kind` and `assurance` are the two **independent** metadata axes the
/// Broker trusts — the dispatch guard reads [`AdapterKind`], the outbox's at-most-once
/// replay reads [`DeliverySemantics`]. Both are asserted separately (never one derived
/// from the other) so a submit mislabeled `AtLeastOnce`, or a read mislabeled
/// `AtMostOnce`, is caught here rather than silently corrupting Broker behavior.
///
/// Whether a `409` must recover is a property of the *kind* (a submit recovers a
/// verified conflict; a read has no recovery semantics), so it is derived from `kind`
/// rather than passed as a third, independently-mistakable flag.
pub fn assert_request_response_contract(
    ctor: AdapterCtor,
    service: &str,
    kind: AdapterKind,
    assurance: DeliverySemantics,
) {
    let recovers_conflict = kind == AdapterKind::Submit;

    // service() is stable.
    let a = ctor(Box::new(ScriptedTransport::new(vec![])));
    assert_eq!(
        a.service(),
        service,
        "adapter reports the wrong service key"
    );

    // kind() is the axis the Broker's dispatch guard reads — a submit reporting
    // `Pull` (or vice versa) would silently defeat it.
    assert_eq!(
        a.kind(),
        kind,
        "adapter reports the wrong kind for its family"
    );

    // assurance() is the SEPARATE axis the outbox's at-most-once replay trusts.
    // Asserting it independently closes the gap where an adapter's kind is right but
    // its delivery semantics are mislabeled (which the dispatch guard cannot catch).
    assert_eq!(
        a.assurance(),
        assurance,
        "adapter reports the wrong delivery semantics"
    );

    // 200 → Accepted with the receipt echoed from the body.
    let a = ctor(Box::new(ScriptedTransport::new(vec![Ok(
        RawResponse::new(200, json!({"receipt": "R-1"})),
    )])));
    match a.call(&env(service), &cred(service)) {
        Ok(AdapterResponse::Accepted { receipt, .. }) => assert_eq!(receipt, "R-1"),
        other => panic!("200 must map to Accepted, got {other:?}"),
    }

    // Timeout → AdapterError::Timeout (the Broker's Unknown depends on this).
    let a = ctor(Box::new(ScriptedTransport::new(vec![Err(
        TransportError::Timeout,
    )])));
    assert!(
        matches!(
            a.call(&env(service), &cred(service)),
            Err(AdapterError::Timeout(_))
        ),
        "a transport timeout must surface as AdapterError::Timeout, never a domain outcome"
    );

    // 5xx: retryable on a pull; ambiguous → AmbiguousDelivery (Unknown) on an
    // AtMostOnce submit. A `503` from a proxy may follow an upstream commit, so a submit
    // must not retry it — and it carries its own `HTTP_<n>` cause code so the outbox
    // trail keeps it distinct from a socket timeout.
    for status in [500u16, 502, 503, 504] {
        let a = ctor(Box::new(ScriptedTransport::new(vec![Ok(
            RawResponse::new(status, json!({})),
        )])));
        let got = a.call(&env(service), &cred(service));
        if recovers_conflict {
            assert!(
                matches!(
                    &got,
                    Err(AdapterError::AmbiguousDelivery { code, detail })
                        if code == &format!("HTTP_{status}")
                            && detail.contains(&status.to_string())
                            && !detail.is_empty()
                ),
                "an AtMostOnce submit must surface {status} as an ambiguous \
                 AmbiguousDelivery carrying its HTTP_{status} cause AND a non-empty \
                 detail naming the status, got {got:?}"
            );
        } else {
            assert!(
                matches!(got, Err(AdapterError::RetryableTransport(_))),
                "a pull must surface {status} as a retryable transport error, got {got:?}"
            );
        }
    }

    // 409 → recovered iff the adapter declares recovery AND the response confirms our
    // idempotency key (this system's key == the envelope's natural_key). Both the
    // `idempotency_key` and the `natural_key` echo field must be honored.
    for field in ["idempotency_key", "natural_key"] {
        let confirmed = json!({"receipt": "R-409", field: "nk-contract"});
        let a = ctor(Box::new(ScriptedTransport::new(vec![Ok(
            RawResponse::new(409, confirmed),
        )])));
        let got = a.call(&env(service), &cred(service));
        if recovers_conflict {
            assert!(
                matches!(got, Ok(AdapterResponse::RecoveredAlreadyProcessed { .. })),
                "a submit adapter must recover a 409 confirmed via {field}, got {got:?}"
            );
        } else {
            assert!(
                matches!(got, Err(AdapterError::Rejected { .. })),
                "a pull adapter has no 409 recovery; it must reject, got {got:?}"
            );
        }
    }

    // An UNVERIFIABLE 409 (no identity echoed) is never a recovery. For a SUBMIT it is
    // AMBIGUOUS — the effect may be committed — so it is `AmbiguousDelivery` (→
    // `Unknown`), NOT a terminal `Rejected` a caller could resubmit under a fresh key.
    // For a pull it is a plain rejection (no recovery semantics).
    let a = ctor(Box::new(ScriptedTransport::new(vec![Ok(
        RawResponse::new(409, json!({"receipt": "R-409"})),
    )])));
    let got = a.call(&env(service), &cred(service));
    if recovers_conflict {
        assert!(
            matches!(
                &got,
                Err(AdapterError::AmbiguousDelivery { code, detail })
                    if code == "HTTP_409" && !detail.is_empty()
            ),
            "an unverifiable 409 on a submit must be ambiguous (AmbiguousDelivery/Unknown) \
             carrying an HTTP_409 cause AND a non-empty detail, got {got:?}"
        );
    } else {
        assert!(
            matches!(got, Err(AdapterError::Rejected { .. })),
            "an unverifiable 409 on a pull must reject, got {got:?}"
        );
    }

    // A 409 that names a DIFFERENT key is a genuine rejection (our request provably not
    // the one on file) — a definitive `Rejected`, for submit and pull alike.
    let a = ctor(Box::new(ScriptedTransport::new(vec![Ok(
        RawResponse::new(409, json!({"idempotency_key": "someone-else"})),
    )])));
    assert!(
        matches!(
            a.call(&env(service), &cred(service)),
            Err(AdapterError::Rejected { .. })
        ),
        "a 409 naming a different key must be a definitive rejection"
    );
}
