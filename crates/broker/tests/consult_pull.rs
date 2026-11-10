//! S5 edge tests: the `consult-llm` Stream adapter and the email/kauppatili pull
//! adapters — the substrate RF1/RF2 build on. Proven through the injected
//! `ScriptedTransport` (no live network in v1).

use serde_json::json;

use aggountant_broker::{
    Broker, BrokerError, ConsultChunk, ConsultLlmAdapter, ConsultTurn, EmailPullAdapter, Envelope,
    PullOutcome, RawResponse, Registry, ScriptedTransport, Transport, TransportError, Vault,
    SVC_CONSULT_LLM, SVC_EMAIL_PULL,
};

fn vault_for(service: &str) -> Vault {
    let mut v = Vault::new();
    v.set(service, "secret");
    v
}

fn consult_broker(home: &std::path::Path, transport: Box<dyn Transport>) -> Broker {
    let mut reg = Registry::new();
    reg.register_stream(Box::new(ConsultLlmAdapter::new(transport)));
    Broker::new(home, vault_for(SVC_CONSULT_LLM), reg)
}

fn pull_broker(home: &std::path::Path, transport: Box<dyn Transport>) -> Broker {
    let mut reg = Registry::new();
    reg.register(Box::new(EmailPullAdapter::new(transport)));
    Broker::new(home, vault_for(SVC_EMAIL_PULL), reg)
}

fn consult_env() -> Envelope {
    Envelope::new(
        SVC_CONSULT_LLM,
        "consult",
        "consult:1",
        json!({"topic": "vat", "question": "deductible?"}),
    )
}

fn pull_env() -> Envelope {
    Envelope::new(
        SVC_EMAIL_PULL,
        "pull-evidence",
        "hunt:email-pull:t1",
        json!({"counterparty": "Acme", "amount": "124.00"}),
    )
}

#[test]
fn consult_maps_final_chunk_to_opaque_advice_body() {
    let home = tempfile::tempdir().unwrap();
    let broker = consult_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            200,
            json!({"type": "final", "advice": {"summary": "reverse-charge", "citations": []}}),
        ))])),
    );
    match broker.consult_turn(&consult_env()).unwrap() {
        ConsultTurn::Chunk(ConsultChunk::Final { body }) => {
            assert_eq!(body["summary"], "reverse-charge");
        }
        other => panic!("expected Final, got {other:?}"),
    }
}

#[test]
fn consult_maps_tool_call_chunk() {
    let home = tempfile::tempdir().unwrap();
    let broker = consult_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            200,
            json!({"type": "tool_call", "tool": "lookup-vat-code", "args": {"x": 1}}),
        ))])),
    );
    match broker.consult_turn(&consult_env()).unwrap() {
        ConsultTurn::Chunk(ConsultChunk::ToolCall { tool, .. }) => {
            assert_eq!(tool, "lookup-vat-code");
        }
        other => panic!("expected ToolCall, got {other:?}"),
    }
}

#[test]
fn consult_surfaces_rate_limit_for_effect_wait() {
    // The load-bearing S5 case: a 429 surfaces as ConsultTurn::RateLimited so the Run can
    // park on a durable Effect::Wait (it is NOT retried in a broker loop).
    let home = tempfile::tempdir().unwrap();
    let broker = consult_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            429,
            json!({"retry_after_secs": 120}),
        ))])),
    );
    match broker.consult_turn(&consult_env()).unwrap() {
        ConsultTurn::RateLimited { retry_after_secs } => {
            assert_eq!(retry_after_secs, Some(120));
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }
}

#[test]
fn consult_uncommon_5xx_is_retryable_not_terminal_malformed() {
    // A stream read has no double-effect hazard, so the WHOLE 5xx class is safely
    // retryable. An uncommon proxy 5xx (507/520/599) must NOT fall through to a terminal
    // MALFORMED (which would kill a retryable consult turn); it surfaces with the
    // RETRYABLE_TRANSPORT cause code, distinct from a genuinely malformed body.
    for status in [507u16, 520, 599] {
        let home = tempfile::tempdir().unwrap();
        let broker = consult_broker(
            home.path(),
            Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
                status,
                json!({}),
            ))])),
        );
        match broker.consult_turn(&consult_env()) {
            Err(BrokerError::Fetch { code, .. }) => assert_eq!(
                code, "RETRYABLE_TRANSPORT",
                "consult {status} must be retryable, not terminal MALFORMED"
            ),
            other => panic!("expected a retryable Fetch error for {status}, got {other:?}"),
        }
    }

    // Contrast: a genuinely malformed 200 body (unknown chunk type) stays terminal.
    let home = tempfile::tempdir().unwrap();
    let broker = consult_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            200,
            json!({"type": "nonsense"}),
        ))])),
    );
    match broker.consult_turn(&consult_env()) {
        Err(BrokerError::Fetch { code, .. }) => assert_eq!(code, "MALFORMED"),
        other => panic!("expected MALFORMED for an unknown chunk type, got {other:?}"),
    }
}

#[test]
fn pull_maps_found_notfound_and_rate_limit() {
    let home = tempfile::tempdir().unwrap();

    // 200 with a document → Found.
    let broker = pull_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            200,
            json!({"document": {"receipt": "R-1"}}),
        ))])),
    );
    match broker.pull(&pull_env()).unwrap() {
        PullOutcome::Found(body) => assert_eq!(body["document"]["receipt"], "R-1"),
        other => panic!("expected Found, got {other:?}"),
    }

    // 404 → NotFound (the adapter's verdict, not a broker status read).
    let broker = pull_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            404,
            json!({}),
        ))])),
    );
    assert_eq!(broker.pull(&pull_env()).unwrap(), PullOutcome::NotFound);

    // 429 → RateLimited (→ Effect::Wait).
    let broker = pull_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            429,
            json!({"retry_after_secs": 30}),
        ))])),
    );
    assert_eq!(
        broker.pull(&pull_env()).unwrap(),
        PullOutcome::RateLimited {
            retry_after_secs: Some(30)
        }
    );
}

#[test]
fn pull_does_not_retry_a_timeout_internally() {
    // A pull is a single attempt — the Run owns the wait/backoff. A timeout is a fetch
    // error, surfaced to the caller (one round trip only).
    let home = tempfile::tempdir().unwrap();
    let scripted = ScriptedTransport::new(vec![Err(TransportError::Timeout)]);
    let mut reg = Registry::new();
    reg.register(Box::new(EmailPullAdapter::new(Box::new(scripted))));
    let broker = Broker::new(home.path(), vault_for(SVC_EMAIL_PULL), reg);
    assert!(broker.pull(&pull_env()).is_err());
}
