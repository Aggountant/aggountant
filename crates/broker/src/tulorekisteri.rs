//! Own-organisation production travel reimbursement channel. Separate from the
//! synthetic payroll adapter, which remains deliberately offline.
use crate::tulorekisteri_xml::{sign_record, soap, status_record, wage_record};
use crate::vero_filing::{VeroCredentialTarget, VeroCredentials};
use crate::{
    AdapterError, AdapterKind, AdapterResponse, Broker, CredentialHandle, DeliverySemantics,
    Envelope, Registry, RequestResponse, Vault,
};
use aggountant_domain::WageReport;
use aggountant_model::BusinessId;
use chrono::{DateTime, Utc};
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{path::Path, time::Duration};
pub const SVC_WAGE: &str = "tulorekisteri-wage";
pub const SVC_STATUS: &str = "tulorekisteri-status";
const BASE: &str = "https://ws.tulorekisteri.fi/20170526/";
const RESPONSE_NS: &str = "http://www.tulorekisteri.fi/2017/1/StatusResponseFromIR";
const SOAP_NS: &str = "http://schemas.xmlsoap.org/soap/envelope/";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WageProfile {
    pub schema_version: u32,
    pub mode: String,
    pub business_id: BusinessId,
    pub reporting_permission_active: bool,
    /// Optional separate credential. Absence uses the configured Vero bundle.
    pub credentials_file: Option<std::path::PathBuf>,
}
pub fn read_wage_profile(home: &Path) -> Result<WageProfile, String> {
    let raw = std::fs::read(home.join(".aggountant/tulorekisteri.json"))
        .map_err(|_| "configure .aggountant/tulorekisteri.json first")?;
    serde_json::from_slice(&raw).map_err(|_| "invalid Tulorekisteri profile".into())
}
pub fn check_readiness(
    profile: &WageProfile,
    handle: &CredentialHandle,
    now: DateTime<Utc>,
) -> Result<(), AdapterError> {
    let fail = || {
        AdapterError::Unauthorized("Tulorekisteri requires schema_version=1, mode=production, reporting_permission_active=true, own-organisation production credential and a currently valid certificate".into())
    };
    if profile.schema_version != 1
        || profile.mode != "production"
        || !profile.reporting_permission_active
    {
        return Err(fail());
    }
    let cred = VeroCredentials::parse(handle.expose_secret()).map_err(|_| fail())?;
    if cred.target != VeroCredentialTarget::Production
        || cred.business_id != profile.business_id
        || cred.authorization_token.is_some()
    {
        return Err(fail());
    }
    let meta = cred.validate_transport_material().map_err(|_| fail())?;
    if now < meta.not_before || now >= meta.not_after {
        return Err(fail());
    }
    Ok(())
}
/// PII lives only in memory. Never persist this request; outbox stores hashes only.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WageRequest {
    pub report: WageReport,
    pub hetu: String,
    pub timestamp: DateTime<Utc>,
}
pub fn wage_envelope(request: &WageRequest, status: bool) -> Result<Envelope, AdapterError> {
    Ok(Envelope::new(
        if status { SVC_STATUS } else { SVC_WAGE },
        if status {
            "GetDeliveryDataStatus"
        } else {
            "SendWageReport"
        },
        request.report.delivery_id(),
        serde_json::to_value(request)
            .map_err(|_| AdapterError::Malformed("invalid wage request".into()))?,
    )
    .with_redaction(crate::RedactionPolicy {
        secret_fields: vec!["hetu".into()],
    }))
}
/// Only this constructor exposes a live broker; no configurable production URL.
pub fn production_broker(
    home: &Path,
    profile: WageProfile,
    secret: String,
    now: DateTime<Utc>,
) -> Result<Broker, AdapterError> {
    let mut vault = Vault::new();
    vault.set(SVC_WAGE, &secret);
    vault.set(SVC_STATUS, secret);
    check_readiness(
        &profile,
        &vault
            .handle(SVC_WAGE)
            .map_err(|_| AdapterError::Unauthorized("missing wage credential".into()))?,
        now,
    )?;
    let mut registry = Registry::new();
    registry.register(Box::new(WageAdapter {
        profile: profile.clone(),
        status: false,
        base: BASE.into(),
        timeout: Duration::from_secs(30),
    }));
    registry.register(Box::new(WageAdapter {
        profile,
        status: true,
        base: BASE.into(),
        timeout: Duration::from_secs(30),
    }));
    Ok(Broker::new(home, vault, registry))
}
struct WageAdapter {
    profile: WageProfile,
    status: bool,
    base: String,
    timeout: Duration,
}
impl RequestResponse for WageAdapter {
    fn service(&self) -> &str {
        if self.status {
            SVC_STATUS
        } else {
            SVC_WAGE
        }
    }
    fn kind(&self) -> AdapterKind {
        if self.status {
            AdapterKind::Pull
        } else {
            AdapterKind::Submit
        }
    }
    fn assurance(&self) -> DeliverySemantics {
        if self.status {
            DeliverySemantics::AtLeastOnce
        } else {
            DeliverySemantics::AtMostOnce
        }
    }
    fn call(
        &self,
        envelope: &Envelope,
        handle: &CredentialHandle,
    ) -> Result<AdapterResponse, AdapterError> {
        check_readiness(&self.profile, handle, Utc::now())?;
        let req: WageRequest = serde_json::from_value(envelope.body.clone())
            .map_err(|_| AdapterError::Malformed("invalid wage request".into()))?;
        if req.report.business_id != self.profile.business_id
            || envelope.natural_key != req.report.delivery_id()
        {
            return Err(AdapterError::Malformed(
                "wage target identity mismatch".into(),
            ));
        }
        let record = if self.status {
            status_record(&req.report, req.timestamp)
        } else {
            wage_record(&req.report, &req.hetu, req.timestamp)?
        };
        let signed = sign_record(&record, handle)?;
        let cred = VeroCredentials::parse(handle.expose_secret())
            .map_err(|_| AdapterError::Unauthorized("invalid credential".into()))?;
        let material = cred
            .transport_material()
            .map_err(AdapterError::Unauthorized)?;
        let client = crate::vero_vat_api::mtls_client(
            material.identity,
            material.trust_certificate,
            self.timeout,
        )?;
        let (path, action) = if self.status {
            ("StatusService.svc", "GetDeliveryDataStatus")
        } else {
            ("WageReportService.svc", "SendWageReport")
        };
        let response = client
            .post(format!("{}{path}", self.base))
            .header("Content-Type", "text/xml;charset=UTF-8")
            .header("SOAPAction", format!("\"{action}\""))
            .body(soap(&signed))
            .send()
            .map_err(|_| unknown("TRANSPORT"))?;
        if !response.status().is_success() {
            return Err(unknown("HTTP_OR_SOAP_FAULT"));
        }
        // Bounded read; no raw response text can enter a log or durable error.
        use std::io::Read;
        let mut bytes = Vec::new();
        response
            .take(1_048_577)
            .read_to_end(&mut bytes)
            .map_err(|_| unknown("RESPONSE_READ"))?;
        if bytes.len() > 1_048_576 {
            return Err(unknown("RESPONSE_SIZE"));
        }
        let body = std::str::from_utf8(&bytes).map_err(|_| unknown("RESPONSE_XML"))?;
        interpret_status(body, &req.report)
    }
}
fn unknown(code: &str) -> AdapterError {
    AdapterError::AmbiguousDelivery {
        code: code.into(),
        detail: "no definitive matching Tulorekisteri feedback; reconcile, never resend".into(),
    }
}
fn child<'a, 'i>(node: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    let mut found = node
        .children()
        .filter(|n| n.has_tag_name(name) && n.tag_name().namespace().is_none());
    let first = found.next()?;
    if found.next().is_some() {
        None
    } else {
        Some(first)
    }
}
fn value<'a, 'i>(node: Node<'a, 'i>, name: &str) -> Option<&'a str> {
    child(node, name)?.text()
}
/// Official IRCommonTypes Guid is a 32-character reference, not a hyphenated UUID.
pub fn ir_reference(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
/// Strict scoped feedback parsing: never find a matching field elsewhere in the
/// document. Ambiguous/malformed/duplicate/pending feedback is NOT acceptance.
pub fn interpret_status(xml: &str, report: &WageReport) -> Result<AdapterResponse, AdapterError> {
    let doc = Document::parse(xml).map_err(|_| unknown("RESPONSE_XML"))?;
    let envelope = doc.root_element();
    if !envelope.has_tag_name((SOAP_NS, "Envelope")) {
        return Err(unknown("SOAP_ENVELOPE"));
    }
    let mut bodies = envelope
        .children()
        .filter(|n| n.has_tag_name((SOAP_NS, "Body")));
    let body = bodies.next().ok_or_else(|| unknown("SOAP_BODY"))?;
    if bodies.next().is_some() {
        return Err(unknown("SOAP_BODY"));
    }
    let roots: Vec<_> = body.children().filter(Node::is_element).collect();
    if roots.len() != 1 || !roots[0].has_tag_name((RESPONSE_NS, "StatusResponseFromIR")) {
        return Err(unknown("SOAP_FAULT_OR_RESPONSE"));
    }
    let root = roots[0];
    let delivery = child(root, "DeliveryData").ok_or_else(|| unknown("DELIVERY_IDENTITY"))?;
    if value(delivery, "DeliveryId") != Some(report.delivery_id().as_str())
        || value(delivery, "DeliveryDataType") != Some("100")
        || value(delivery, "ProductionEnvironment") != Some("true")
    {
        return Err(unknown("DELIVERY_IDENTITY"));
    }
    for n in [
        "DeliveryDataOwner",
        "DeliveryDataCreator",
        "DeliveryDataSender",
    ] {
        let actor = child(delivery, n).ok_or_else(|| unknown("DELIVERY_IDENTITY"))?;
        if value(actor, "Type") != Some("1")
            || value(actor, "Code") != Some(report.business_id.as_str())
        {
            return Err(unknown("DELIVERY_IDENTITY"));
        }
    }
    let status = child(root, "StatusResponse").ok_or_else(|| unknown("RESPONSE_STATUS"))?;
    let codes: Vec<_> = status
        .descendants()
        .filter(|n| n.has_tag_name("ErrorCode"))
        .filter_map(|n| n.text())
        .collect();
    if codes.contains(&"DDVS0280") {
        return Err(unknown("DDVS0280"));
    }
    match value(status, "DeliveryDataStatus") {
        Some("3") => {
            if !codes.is_empty() || child(status, "InvalidItems").is_some() {
                return Err(unknown("CONTRADICTORY_FEEDBACK"));
            }
            let valid = child(status, "ValidItems").ok_or_else(|| unknown("VALID_ITEMS"))?;
            let items: Vec<_> = valid.children().filter(Node::is_element).collect();
            if items.len() != 1
                || !items[0].has_tag_name("Item")
                || value(items[0], "ItemId") != Some(report.report_id().as_str())
                || value(items[0], "ItemVersion") != Some("1")
            {
                return Err(unknown("REPORT_IDENTITY"));
            }
            let remote = value(items[0], "IRItemId")
                .filter(|s| ir_reference(s))
                .ok_or_else(|| unknown("REPORT_REFERENCE"))?;
            let delivery = value(status, "IRDeliveryId")
                .filter(|s| ir_reference(s))
                .ok_or_else(|| unknown("DELIVERY_REFERENCE"))?;
            Ok(AdapterResponse::Accepted {
                receipt: format!("{delivery}/{remote}/1"),
                body: json!({"delivery_id":report.delivery_id(),"report_id":report.report_id(),"ir_delivery_id":delivery,"ir_report_id":remote,"version":1}),
            })
        }
        Some("4" | "5") => Err(AdapterError::Rejected {
            code: "TULOREKISTERI_REJECTED".into(),
            message: "authority rejected the matching delivery; inspect feedback in the e-service"
                .into(),
        }),
        _ => Err(unknown("PENDING_OR_UNKNOWN")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        tulorekisteri_xml::{record_hash, STATUS_NS, WAGE_NS},
        RemoteOutcome,
    };
    use aggountant_vero_vat_api_mock::{
        MockVeroVatServer, Scenario, SoapScenario, WageTestCredentials as Pki,
    };
    pub fn report() -> WageReport {
        serde_json::from_value(json!({"business_id":"1234567-1","employee_id":"fictional-employee","source_reference":"fictional-expense-2026-09","payment_date":"2026-10-25","period_start":"2026-09-01","period_end":"2026-09-30","joint_owner":true,"contact_name":"Fictional Contact","contact_phone":"+358 40 1234567","lines":[{"income_type":"311","amount":"120.00","kilometers":218},{"income_type":"331","amount":"54.00","allowance_code":3}]})).unwrap()
    }
    fn time() -> DateTime<Utc> {
        "2026-10-25T12:00:00Z".parse().unwrap()
    }
    fn secret() -> String {
        json!({"target":"production","business_id":"1234567-1","software_key":"fictional","client_certificate_pem":Pki::CERT,"client_private_key_pem":Pki::KEY,"ca_certificate_pem":Pki::CA}).to_string()
    }
    fn profile() -> WageProfile {
        WageProfile {
            schema_version: 1,
            mode: "production".into(),
            business_id: BusinessId::parse("1234567-1").unwrap(),
            reporting_permission_active: true,
            credentials_file: None,
        }
    }
    fn feedback(status: &str, extra: &str) -> String {
        let r = report();
        let actors = [
            "DeliveryDataOwner",
            "DeliveryDataCreator",
            "DeliveryDataSender",
        ]
        .iter()
        .map(|n| format!("<{n}><Type>1</Type><Code>1234567-1</Code></{n}>"))
        .collect::<String>();
        soap(&format!("<ir:StatusResponseFromIR xmlns:ir=\"{RESPONSE_NS}\"><DeliveryData><DeliveryDataType>100</DeliveryDataType><DeliveryId>{}</DeliveryId><ProductionEnvironment>true</ProductionEnvironment>{actors}</DeliveryData><StatusResponse><DeliveryDataStatus>{status}</DeliveryDataStatus><IRDeliveryId>11111111222233334444555555555555</IRDeliveryId>{extra}</StatusResponse></ir:StatusResponseFromIR>",r.delivery_id()))
    }
    fn accepted() -> String {
        feedback("3",&format!("<ValidItems><Item><ItemId>{}</ItemId><IRItemId>22222222222233334444555555555555</IRItemId><ItemVersion>1</ItemVersion></Item></ValidItems>",report().report_id()))
    }
    fn request() -> WageRequest {
        WageRequest {
            report: report(),
            hetu: "010101A123N".into(),
            timestamp: time(),
        }
    }
    fn handle() -> CredentialHandle {
        let mut v = Vault::new();
        v.set(SVC_WAGE, secret());
        v.handle(SVC_WAGE).unwrap()
    }
    fn mock(body: String, http: u16, delay: Duration) -> MockVeroVatServer {
        let mut s = Scenario::file_receipt("wage-soap");
        s.soap = Some(SoapScenario {
            send_body: body,
            send_status: http,
            send_delay: delay,
            status_body: accepted(),
        });
        MockVeroVatServer::start_wage_mtls(s).unwrap()
    }
    fn broker(home: &Path, mock: &MockVeroVatServer, timeout: Duration) -> Broker {
        let mut registry = Registry::new();
        let mut vault = Vault::new();
        for status in [false, true] {
            let adapter = WageAdapter {
                profile: profile(),
                status,
                base: format!("{}/20170526/", mock.base_url()),
                timeout,
            };
            vault.set(adapter.service(), secret());
            registry.register(Box::new(adapter));
        }
        Broker::new(home, vault, registry)
    }
    #[test]
    fn mtls_accepted_and_signed_status_read() {
        let m = mock(accepted(), 200, Duration::ZERO);
        let h = tempfile::tempdir().unwrap();
        let b = broker(h.path(), &m, Duration::from_secs(2));
        let r = request();
        assert!(b
            .submit("test", "effect", &wage_envelope(&r, false).unwrap())
            .unwrap()
            .is_terminal_success());
        let status = b.fetch(&wage_envelope(&r, true).unwrap()).unwrap();
        assert_eq!(status["version"], 1);
        let requests = m.requests();
        assert_eq!(requests.len(), 2);
        for (i, q) in requests.iter().enumerate() {
            assert_eq!(
                q.headers["soapaction"],
                if i == 0 {
                    "\"SendWageReport\""
                } else {
                    "\"GetDeliveryDataStatus\""
                }
            );
            assert_eq!(q.headers["content-type"], "text/xml;charset=UTF-8");
            assert!(!q.headers.contains_key("vero-softwarekey"));
            let s = std::str::from_utf8(&q.body).unwrap();
            assert!(s.contains("<ds:Signature"));
            assert!(s.contains(if i == 0 { WAGE_NS } else { STATUS_NS }));
        }
    }
    #[test]
    fn mtls_rejected_duplicate_fault_timeout_and_5xx_are_honest() {
        for (body,http,delay,rejected) in [
            (feedback("4",""),200,Duration::ZERO,true),
            (feedback("4","<DeliveryErrors><ErrorInfo><ErrorCode>DDVS0280</ErrorCode><ErrorMessage>PII must not escape</ErrorMessage></ErrorInfo></DeliveryErrors>"),200,Duration::ZERO,false),
            (soap("<soap:Fault><faultstring>secret</faultstring></soap:Fault>"),200,Duration::ZERO,false),
            (accepted(),200,Duration::from_millis(250),false),
            (accepted(),503,Duration::ZERO,false),
        ] {
            let m=mock(body,http,delay);let h=tempfile::tempdir().unwrap();let b=broker(h.path(),&m,Duration::from_millis(100));
            let r=request();let outcome=b.submit("test","effect",&wage_envelope(&r,false).unwrap()).unwrap();
            if rejected { assert!(matches!(outcome,RemoteOutcome::Rejected{..})); } else { assert_eq!(outcome,RemoteOutcome::Unknown); }
            assert_eq!(m.requests().iter().filter(|q|q.path.ends_with("WageReportService.svc")).count(),1);
            assert!(b.fetch(&wage_envelope(&r,true).unwrap()).is_ok());
            let outbox=std::fs::read(h.path().join("integration/outbox/outbox.db")).unwrap(); let outbox=String::from_utf8_lossy(&outbox); for secret in ["PII must not escape", "010101A123N", "PRIVATE KEY"] { assert!(!outbox.contains(secret)); }
        }
    }
    #[test]
    fn malformed_pending_and_mismatching_feedback_never_accepts() {
        for body in [
            accepted().replace(&report().report_id(), "wrong"),
            accepted().replace("1234567-1", "7654321-2"),
            accepted().replace("<ItemVersion>1", "<ItemVersion>2"),
            feedback("2", ""),
            feedback("0", ""),
            "not xml".into(),
            accepted().replace("<DeliveryDataStatus>3", "<DeliveryDataStatus>6"),
        ] {
            assert!(matches!(
                interpret_status(&body, &report()),
                Err(AdapterError::AmbiguousDelivery { .. })
            ));
        }
    }
    #[test]
    fn readiness_fails_closed() {
        let h = handle();
        check_readiness(&profile(), &h, time()).unwrap();
        let mut p = profile();
        p.reporting_permission_active = false;
        assert!(check_readiness(&p, &h, time()).is_err());
        p = profile();
        p.mode = "test".into();
        assert!(check_readiness(&p, &h, time()).is_err());
        p = profile();
        p.business_id = BusinessId::parse("7654321-2").unwrap();
        assert!(check_readiness(&p, &h, time()).is_err());
        assert!(check_readiness(&profile(), &h, "2040-01-01T00:00:00Z".parse().unwrap()).is_err());
    }
    #[test]
    fn golden_signed_records_and_rsa_verification() {
        let r = request();
        let h = handle();
        for (name, unsigned) in [
            (
                "wage",
                wage_record(&r.report, &r.hetu, r.timestamp).unwrap(),
            ),
            ("status", status_record(&r.report, r.timestamp)),
        ] {
            let signed = sign_record(&unsigned, &h).unwrap();
            let doc = Document::parse(&signed).unwrap();
            let si = doc
                .descendants()
                .find(|n| n.tag_name().name() == "SignedInfo")
                .unwrap();
            let sig = doc
                .descendants()
                .find(|n| n.tag_name().name() == "SignatureValue")
                .unwrap()
                .text()
                .unwrap();
            use base64::{engine::general_purpose::STANDARD, Engine};
            use ring::signature;
            let (_, pem) = x509_parser::pem::parse_x509_pem(Pki::CERT.as_bytes()).unwrap();
            let (_, cert) = x509_parser::parse_x509_certificate(&pem.contents).unwrap();
            let info = signed[si.range()].replace(
                "<ds:SignedInfo>",
                "<ds:SignedInfo xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\">",
            );
            signature::UnparsedPublicKey::new(
                &signature::RSA_PKCS1_2048_8192_SHA256,
                cert.public_key().subject_public_key.data.as_ref(),
            )
            .verify(info.as_bytes(), &STANDARD.decode(sig).unwrap())
            .unwrap();
            let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/{name}-signed.xml"));
            if std::env::var_os("UPDATE_WAGE_GOLDEN").is_some() {
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, &signed).unwrap();
            }
            assert_eq!(signed, std::fs::read_to_string(path).unwrap());
            assert_eq!(record_hash(&unsigned).len(), 64);
        }
    }
    #[test]
    fn official_references_are_32_hex_not_hyphenated_uuid() {
        assert!(ir_reference("850166cc02fa4a038da5ee36b990b07a"));
        for bad in [
            "850166cc-02fa-4a03-8da5-ee36b990b07a",
            "850166cc02fa4a038da5ee36b990b07z",
            "",
        ] {
            assert!(!ir_reference(bad));
        }
    }
}
