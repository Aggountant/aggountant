//! Own-organisation production Incomes Register channel: the travel-only earnings
//! payment report and the monthly employer's separate report. Separate from the
//! synthetic payroll adapter, which remains deliberately offline.
use crate::tulorekisteri_xml::{
    invalidation_record, payer_summary_record, sign_record, soap, status_record, wage_record,
    wage_replacement_record, DELIVERY_TYPE_EMPLOYER, DELIVERY_TYPE_WAGE,
    DELIVERY_TYPE_WAGE_CANCELLATION,
};
use crate::vero_filing::{VeroCertificateMetadata, VeroCredentialTarget, VeroCredentials};
use crate::{
    AdapterError, AdapterKind, AdapterResponse, Broker, CredentialHandle, DeliverySemantics,
    Envelope, Registry, RequestResponse, Vault,
};
use aggountant_domain::{
    payroll::{EmployerReportAction, EmployerSeparateReport, PensionInsurance, ReportMonth},
    WageCorrection, WageCorrectionAction, WageReport,
};
use aggountant_model::{BusinessId, Instant};
use aggountant_model::{FilingContactName, FilingContactPhone};
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{path::Path, time::Duration};
pub const SVC_WAGE: &str = "tulorekisteri-wage";
pub const SVC_STATUS: &str = "tulorekisteri-status";
pub const SVC_EMPLOYER: &str = "tulorekisteri-employer-report";
pub const SVC_EMPLOYER_STATUS: &str = "tulorekisteri-employer-status";
pub const SVC_WAGE_CORRECTION: &str = "tulorekisteri-wage-correction";
pub const SVC_WAGE_CORRECTION_STATUS: &str = "tulorekisteri-wage-correction-status";
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
    /// Settings of the monthly employer's separate report. Absent until the
    /// company is in the Tax Administration's employer register.
    pub employer_report: Option<EmployerReportProfile>,
}
/// Company settings for the employer's separate report.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmployerReportProfile {
    /// First month of the employer-register entry: a report is due for every
    /// month from this one on, also when nothing was paid.
    pub registered_from: ReportMonth,
    pub contact_name: FilingContactName,
    pub contact_phone: FilingContactPhone,
    /// Earnings-related pension insurances sent with "No wages payable". Empty
    /// when the payer has no obligation to insure (for example only YEL-insured
    /// owners); stated explicitly so an omission is a decision, not an accident.
    pub pension_insurances: Vec<PensionInsurance>,
}
impl EmployerReportProfile {
    /// What the separate report needs from the settings beyond their types
    /// (`ReportMonth` already refuses a month before 2019-01): every pension
    /// insurance has the shape the report schema requires.
    pub fn validate(&self) -> Result<(), String> {
        if self.pension_insurances.len() > 20 {
            return Err("at most 20 pension insurances".into());
        }
        for p in &self.pension_insurances {
            p.validate().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

pub fn wage_profile_path(home: &Path) -> std::path::PathBuf {
    home.join(".aggountant/tulorekisteri.json")
}
pub fn read_wage_profile(home: &Path) -> Result<WageProfile, String> {
    let raw = std::fs::read(wage_profile_path(home)).map_err(|_| {
        "no Tulorekisteri profile; write it with `aggountant tulorekisteri configure`"
    })?;
    parse_wage_profile(&raw)
}
fn parse_wage_profile(raw: &[u8]) -> Result<WageProfile, String> {
    serde_json::from_slice(raw).map_err(|e| format!("invalid Tulorekisteri profile: {e}"))
}

/// The profile with its `employer_report` block replaced.
pub struct SplicedProfile {
    /// The new file text.
    pub text: String,
    /// The new profile, parsed back from `text`.
    pub profile: WageProfile,
    /// The block that was there, as written (it need not have been valid).
    pub before: Option<serde_json::Value>,
}

/// The profile text with its `employer_report` block replaced by `settings`, or
/// added as the last member when absent. Every other byte stays as it was, so
/// a hand-kept layout, key order and the other settings are untouched. An
/// existing block that no longer parses is replaced too, so a bad hand edit
/// can be repaired; the rest of the profile must be valid.
pub fn with_employer_report(
    raw: &str,
    settings: &EmployerReportProfile,
) -> Result<SplicedProfile, String> {
    let members: std::collections::BTreeMap<String, &serde_json::value::RawValue> =
        serde_json::from_str(raw).map_err(|_| "invalid Tulorekisteri profile")?;
    let block = serde_json::to_string_pretty(settings)
        .map_err(|_| "employer_report serialization failed")?
        .replace('\n', "\n  ");
    let existing = members.get("employer_report");
    let text = match existing {
        Some(value) => {
            // The value borrows from `raw`, so its offset is the pointer distance.
            let start = value.get().as_ptr() as usize - raw.as_ptr() as usize;
            let end = start + value.get().len();
            if raw.get(start..end) != Some(value.get()) {
                return Err("employer_report could not be located in the profile".into());
            }
            format!("{}{block}{}", &raw[..start], &raw[end..])
        }
        None => {
            let close = raw
                .trim_end()
                .len()
                .checked_sub(1)
                .filter(|&i| raw[i..].starts_with('}'))
                .ok_or("invalid Tulorekisteri profile")?;
            let last = raw[..close].trim_end().len();
            format!(
                "{},\n  \"employer_report\": {block}{}",
                &raw[..last],
                &raw[last..]
            )
        }
    };
    // Also refuses a duplicated key, which the map above would hide.
    let profile = parse_wage_profile(text.as_bytes())?;
    let value = |v: Option<&EmployerReportProfile>| serde_json::to_value(v).ok();
    if value(profile.employer_report.as_ref()) != value(Some(settings)) {
        return Err("employer_report did not round-trip through the profile".into());
    }
    let before = existing
        .map(|v| serde_json::from_str(v.get()))
        .transpose()
        .map_err(|_| "invalid Tulorekisteri profile")?;
    Ok(SplicedProfile {
        text,
        profile,
        before,
    })
}

/// Replace the profile file atomically with `bytes` (already validated).
pub fn write_wage_profile(home: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let path = wage_profile_path(home);
    let temporary = path.with_extension("json.tmp");
    let fail = |what: &str| format!("Tulorekisteri profile could not be {what}");
    {
        let mut file = std::fs::File::create(&temporary).map_err(|_| fail("created"))?;
        file.write_all(bytes).map_err(|_| fail("written"))?;
        file.sync_all().map_err(|_| fail("synced"))?;
    }
    std::fs::rename(&temporary, &path).map_err(|_| fail("replaced"))
}
/// One requirement of the channel that is not met: a stable code, who settles it
/// (`human`: the user's fact, confirmation or e-service step; `agent`: a setting
/// the agent can fix) and what to do.
#[derive(Debug, Clone, Serialize)]
pub struct TulorekisteriBlocker {
    pub code: &'static str,
    pub owner: &'static str,
    pub message: String,
}

/// Offline readiness of the own-organisation Incomes Register channel, itemised.
/// The same requirements gate every send ([`check_readiness`]).
#[derive(Debug, Clone, Serialize)]
pub struct TulorekisteriReadiness {
    pub ready: bool,
    pub profile_path: String,
    pub mode: Option<String>,
    pub business_id: Option<String>,
    pub reporting_permission_active: Option<bool>,
    pub credential_path: Option<String>,
    pub certificate: Option<VeroCertificateMetadata>,
    /// Whether the employer's separate report settings are written; not a
    /// requirement of the travel report.
    pub employer_report_configured: bool,
    pub blockers: Vec<TulorekisteriBlocker>,
}

fn blocker(
    blockers: &mut Vec<TulorekisteriBlocker>,
    code: &'static str,
    owner: &'static str,
    message: impl Into<String>,
) {
    blockers.push(TulorekisteriBlocker {
        code,
        owner,
        message: message.into(),
    });
}

/// What the profile itself must state.
fn profile_blockers(profile: &WageProfile, blockers: &mut Vec<TulorekisteriBlocker>) {
    if profile.schema_version != 1 {
        blocker(
            blockers,
            "TULOREKISTERI_SCHEMA_VERSION_UNSUPPORTED",
            "agent",
            format!(
                "profile schema_version is {}, this binary reads 1; write it with `tulorekisteri configure`",
                profile.schema_version
            ),
        );
    }
    if profile.mode != "production" {
        blocker(
            blockers,
            "TULOREKISTERI_DISABLED",
            "human",
            format!(
                "profile mode is {:?}; the channel sends only in mode \"production\", set on the user's say-so",
                profile.mode
            ),
        );
    }
    if !profile.reporting_permission_active {
        blocker(
            blockers,
            "TULOREKISTERI_REPORTING_PERMISSION_UNATTESTED",
            "human",
            "the user has not confirmed that the Incomes Register wage reporting permission is granted (reporting_permission_active)",
        );
    }
}

/// What the credential bundle must satisfy for this profile at `now`. Returns
/// the certificate metadata when the bundle is readable.
fn credential_blockers(
    profile: &WageProfile,
    secret: &str,
    now: Instant,
    blockers: &mut Vec<TulorekisteriBlocker>,
) -> Option<VeroCertificateMetadata> {
    let cred = match VeroCredentials::parse(secret) {
        Ok(cred) => cred,
        Err(message) => {
            blocker(
                blockers,
                "TULOREKISTERI_CREDENTIAL_INVALID",
                "agent",
                message,
            );
            return None;
        }
    };
    if cred.target != VeroCredentialTarget::Production {
        blocker(
            blockers,
            "TULOREKISTERI_CREDENTIAL_TARGET_MISMATCH",
            "agent",
            "the credential bundle is not for the production target",
        );
    }
    if cred.business_id != profile.business_id {
        blocker(
            blockers,
            "TULOREKISTERI_CREDENTIAL_BUSINESS_ID_MISMATCH",
            "human",
            format!(
                "the credential bundle is for Business ID {}, the profile states {}",
                cred.business_id.as_str(),
                profile.business_id.as_str()
            ),
        );
    }
    if cred.authorization_token.is_some() {
        blocker(
            blockers,
            "TULOREKISTERI_DELEGATED_USE_UNSUPPORTED",
            "human",
            "the credential bundle carries an authorization_token; reporting on behalf of another organisation is not supported",
        );
    }
    let meta = match cred.validate_transport_material() {
        Ok(meta) => meta,
        Err(message) => {
            blocker(
                blockers,
                "TULOREKISTERI_CREDENTIAL_INVALID",
                "agent",
                message,
            );
            return None;
        }
    };
    if crate::tulorekisteri_xml::signing_key(&cred).is_err() {
        blocker(
            blockers,
            "TULOREKISTERI_CREDENTIAL_INVALID",
            "agent",
            "the client private key is not an RSA key in PKCS#8 or PKCS#1 PEM, which record signing needs",
        );
    }
    if now < meta.not_before {
        blocker(
            blockers,
            "TULOREKISTERI_CERTIFICATE_NOT_YET_VALID",
            "human",
            format!("the certificate is valid from {}", meta.not_before),
        );
    }
    if now >= meta.not_after {
        blocker(
            blockers,
            "TULOREKISTERI_CERTIFICATE_EXPIRED",
            "human",
            format!("the certificate expired at {}", meta.not_after),
        );
    }
    Some(meta)
}

/// Read the profile: `Ok(None)` when the file does not exist.
pub fn load_wage_profile(home: &Path) -> Result<Option<WageProfile>, String> {
    match std::fs::read(wage_profile_path(home)) {
        Ok(raw) => parse_wage_profile(&raw).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("the Tulorekisteri profile could not be read".into()),
    }
}

/// Offline, itemised readiness: the profile as read, the credential file at
/// `credential_path` (the profile's `credentials_file`, else the Vero bundle) and
/// the certificate's validity at `now`. Makes no network request.
pub fn evaluate_readiness(
    home: &Path,
    profile: Result<Option<WageProfile>, String>,
    credential_path: Option<&Path>,
    now: Instant,
) -> TulorekisteriReadiness {
    let mut blockers = Vec::new();
    let profile = match profile {
        Ok(Some(profile)) => Some(profile),
        Ok(None) => {
            blocker(
                &mut blockers,
                "TULOREKISTERI_PROFILE_MISSING",
                "agent",
                "there is no Tulorekisteri profile; write it with `tulorekisteri configure`",
            );
            None
        }
        Err(message) => {
            blocker(
                &mut blockers,
                "TULOREKISTERI_PROFILE_INVALID",
                "agent",
                format!("{message}; rewrite it with `tulorekisteri configure --replace`"),
            );
            None
        }
    };
    let mut certificate = None;
    if let Some(profile) = &profile {
        profile_blockers(profile, &mut blockers);
        match credential_path {
            None => blocker(
                &mut blockers,
                "TULOREKISTERI_CREDENTIAL_PATH_MISSING",
                "agent",
                "no credential file: set the profile's credentials_file or the Vero credential path (`vero-fi configure`)",
            ),
            Some(path) if !path.is_absolute() => blocker(
                &mut blockers,
                "TULOREKISTERI_CREDENTIAL_PATH_NOT_ABSOLUTE",
                "agent",
                format!("credential path {} is not absolute", path.display()),
            ),
            Some(path) => {
                let inside_home = match (path.canonicalize(), home.canonicalize()) {
                    (Ok(path), Ok(home)) => path.starts_with(home),
                    _ => path.starts_with(home),
                };
                if inside_home {
                    blocker(
                        &mut blockers,
                        "TULOREKISTERI_CREDENTIAL_IN_ACCOUNTING_HOME",
                        "agent",
                        "the credential file must be outside the git-versioned accounting home",
                    );
                } else {
                    match std::fs::read_to_string(path) {
                        Ok(secret) => {
                            certificate = credential_blockers(profile, &secret, now, &mut blockers)
                        }
                        Err(_) => blocker(
                            &mut blockers,
                            "TULOREKISTERI_CREDENTIAL_UNAVAILABLE",
                            "agent",
                            format!("credential file {} is not readable", path.display()),
                        ),
                    }
                }
            }
        }
    }
    TulorekisteriReadiness {
        ready: blockers.is_empty(),
        profile_path: wage_profile_path(home).display().to_string(),
        mode: profile.as_ref().map(|p| p.mode.clone()),
        business_id: profile.as_ref().map(|p| p.business_id.as_str().to_string()),
        reporting_permission_active: profile.as_ref().map(|p| p.reporting_permission_active),
        credential_path: credential_path.map(|p| p.display().to_string()),
        certificate,
        employer_report_configured: profile
            .as_ref()
            .is_some_and(|p| p.employer_report.is_some()),
        blockers,
    }
}

/// The send-time gate: the same requirements as [`evaluate_readiness`] for a
/// credential already read, refused with every unmet one named.
pub fn check_readiness(
    profile: &WageProfile,
    handle: &CredentialHandle,
    now: Instant,
) -> Result<(), AdapterError> {
    let mut blockers = Vec::new();
    profile_blockers(profile, &mut blockers);
    credential_blockers(profile, handle.expose_secret(), now, &mut blockers);
    if blockers.is_empty() {
        return Ok(());
    }
    let items: Vec<String> = blockers
        .iter()
        .map(|b| format!("{} ({}): {}", b.code, b.owner, b.message))
        .collect();
    Err(AdapterError::Unauthorized(format!(
        "Tulorekisteri channel not ready: {}; `aggountant tulorekisteri status` lists them offline",
        items.join("; ")
    )))
}
/// PII lives only in memory. Never persist this request; outbox stores hashes only.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WageRequest {
    pub report: WageReport,
    pub hetu: String,
    pub timestamp: Instant,
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
/// One correction (replacement or cancellation) of an accepted travel report, as
/// sent at `timestamp`. `hetu` is needed only to send a replacement; it is empty
/// for a cancellation and for every status query. For a status query
/// `status_timestamp` is the query time; the record identity still comes from the
/// correction and its send `timestamp`. Never persist this request.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WageCorrectionRequest {
    pub correction: WageCorrection,
    pub hetu: String,
    pub timestamp: Instant,
    pub status_timestamp: Option<Instant>,
}
pub fn wage_correction_envelope(
    request: &WageCorrectionRequest,
    status: bool,
) -> Result<Envelope, AdapterError> {
    Ok(Envelope::new(
        if status {
            SVC_WAGE_CORRECTION_STATUS
        } else {
            SVC_WAGE_CORRECTION
        },
        match (status, request.correction.action) {
            (true, _) => "GetDeliveryDataStatus",
            (false, WageCorrectionAction::Replacement) => "SendWageReport",
            (false, WageCorrectionAction::Cancellation) => "SendInvalidation",
        },
        request.correction.delivery_id(request.timestamp),
        serde_json::to_value(request)
            .map_err(|_| AdapterError::Malformed("invalid wage correction request".into()))?,
    )
    .with_redaction(crate::RedactionPolicy {
        secret_fields: vec!["hetu".into()],
    }))
}
/// One employer's separate report as sent at `timestamp`. Carries no PII.
/// For a status query `status_timestamp` is the query time; the record identity
/// still comes from the report and its send `timestamp`.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmployerReportRequest {
    pub report: EmployerSeparateReport,
    pub timestamp: Instant,
    pub status_timestamp: Option<Instant>,
}
pub fn employer_envelope(
    request: &EmployerReportRequest,
    status: bool,
) -> Result<Envelope, AdapterError> {
    Ok(Envelope::new(
        if status {
            SVC_EMPLOYER_STATUS
        } else {
            SVC_EMPLOYER
        },
        if status {
            "GetDeliveryDataStatus"
        } else {
            "SendPayerSummaryReport"
        },
        request.report.delivery_id(request.timestamp),
        serde_json::to_value(request)
            .map_err(|_| AdapterError::Malformed("invalid employer report request".into()))?,
    ))
}
/// Only this constructor exposes a live broker; no configurable production URL.
pub fn production_broker(
    home: &Path,
    profile: WageProfile,
    secret: String,
    now: Instant,
) -> Result<Broker, AdapterError> {
    let mut vault = Vault::new();
    for op in Op::ALL {
        vault.set(op.service(), &secret);
    }
    check_readiness(
        &profile,
        &vault
            .handle(SVC_WAGE)
            .map_err(|_| AdapterError::Unauthorized("missing wage credential".into()))?,
        now,
    )?;
    let mut registry = Registry::new();
    for op in Op::ALL {
        registry.register(Box::new(TulorekisteriAdapter {
            profile: profile.clone(),
            op,
            base: BASE.into(),
            timeout: Duration::from_secs(30),
        }));
    }
    Ok(Broker::new(home, vault, registry))
}
/// The operations of the own-organisation channel. Each is its own broker
/// service so the outbox keys sends and status reads separately.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Wage,
    WageStatus,
    Employer,
    EmployerStatus,
    WageCorrection,
    WageCorrectionStatus,
}
/// Service path and SOAPAction, from the official WSDLs.
type Endpoint = (&'static str, &'static str);
const WAGE_ENDPOINT: Endpoint = ("WageReportService.svc", "SendWageReport");
const STATUS_ENDPOINT: Endpoint = ("StatusService.svc", "GetDeliveryDataStatus");
impl Op {
    const ALL: [Op; 6] = [
        Op::Wage,
        Op::WageStatus,
        Op::Employer,
        Op::EmployerStatus,
        Op::WageCorrection,
        Op::WageCorrectionStatus,
    ];
    fn service(self) -> &'static str {
        match self {
            Op::Wage => SVC_WAGE,
            Op::WageStatus => SVC_STATUS,
            Op::Employer => SVC_EMPLOYER,
            Op::EmployerStatus => SVC_EMPLOYER_STATUS,
            Op::WageCorrection => SVC_WAGE_CORRECTION,
            Op::WageCorrectionStatus => SVC_WAGE_CORRECTION_STATUS,
        }
    }
    fn is_status(self) -> bool {
        matches!(
            self,
            Op::WageStatus | Op::EmployerStatus | Op::WageCorrectionStatus
        )
    }
}
struct TulorekisteriAdapter {
    profile: WageProfile,
    op: Op,
    base: String,
    timeout: Duration,
}
impl TulorekisteriAdapter {
    /// Rebuild the record from validated input (never sign caller-supplied XML)
    /// and say which feedback item would prove acceptance.
    fn record(
        &self,
        envelope: &Envelope,
    ) -> Result<(String, ExpectedItem, Endpoint), AdapterError> {
        let mismatch = || AdapterError::Malformed("Tulorekisteri target identity mismatch".into());
        match self.op {
            Op::Wage | Op::WageStatus => {
                let req: WageRequest = serde_json::from_value(envelope.body.clone())
                    .map_err(|_| AdapterError::Malformed("invalid wage request".into()))?;
                if req.report.business_id != self.profile.business_id
                    || envelope.natural_key != req.report.delivery_id()
                {
                    return Err(mismatch());
                }
                let expected = ExpectedItem::wage(&req.report);
                Ok(if self.op.is_status() {
                    let record = status_record(
                        DELIVERY_TYPE_WAGE,
                        &expected.delivery_id,
                        &expected.business_id,
                        req.timestamp,
                    );
                    (record, expected, STATUS_ENDPOINT)
                } else {
                    let record = wage_record(&req.report, &req.hetu, req.timestamp)?;
                    (record, expected, WAGE_ENDPOINT)
                })
            }
            Op::WageCorrection | Op::WageCorrectionStatus => {
                let malformed =
                    || AdapterError::Malformed("invalid wage correction request".into());
                let req: WageCorrectionRequest =
                    serde_json::from_value(envelope.body.clone()).map_err(|_| malformed())?;
                let c = &req.correction;
                if c.report.business_id != self.profile.business_id
                    || envelope.natural_key != c.delivery_id(req.timestamp)
                {
                    return Err(mismatch());
                }
                c.validate(req.timestamp.utc_date())
                    .map_err(|_| malformed())?;
                let expected = ExpectedItem::wage_correction(c, req.timestamp);
                Ok(
                    match (self.op.is_status(), req.status_timestamp, c.action) {
                        (true, Some(at), _) => {
                            let record = status_record(
                                expected.delivery_data_type,
                                &expected.delivery_id,
                                &expected.business_id,
                                at,
                            );
                            (record, expected, STATUS_ENDPOINT)
                        }
                        (false, None, WageCorrectionAction::Replacement) => {
                            let record = wage_replacement_record(c, &req.hetu, req.timestamp)?;
                            (record, expected, WAGE_ENDPOINT)
                        }
                        (false, None, WageCorrectionAction::Cancellation) => {
                            let record = invalidation_record(
                                DELIVERY_TYPE_WAGE_CANCELLATION,
                                &expected.delivery_id,
                                &expected.business_id,
                                &c.ir_report_id,
                                c.corrects_version,
                                req.timestamp,
                            );
                            (
                                record,
                                expected,
                                ("InvalidationService.svc", "SendInvalidation"),
                            )
                        }
                        _ => return Err(malformed()),
                    },
                )
            }
            Op::Employer | Op::EmployerStatus => {
                let req: EmployerReportRequest = serde_json::from_value(envelope.body.clone())
                    .map_err(|_| {
                        AdapterError::Malformed("invalid employer report request".into())
                    })?;
                if req.report.business_id != self.profile.business_id
                    || envelope.natural_key != req.report.delivery_id(req.timestamp)
                {
                    return Err(mismatch());
                }
                let expected = ExpectedItem::employer(&req.report, req.timestamp);
                Ok(match (self.op.is_status(), req.status_timestamp) {
                    (true, Some(at)) => {
                        let record = status_record(
                            DELIVERY_TYPE_EMPLOYER,
                            &expected.delivery_id,
                            &expected.business_id,
                            at,
                        );
                        (record, expected, STATUS_ENDPOINT)
                    }
                    (false, None) => (
                        payer_summary_record(&req.report, req.timestamp)?,
                        expected,
                        ("PayerSummaryReportService.svc", "SendPayerSummaryReport"),
                    ),
                    _ => {
                        return Err(AdapterError::Malformed(
                            "invalid employer report request".into(),
                        ))
                    }
                })
            }
        }
    }
}
impl RequestResponse for TulorekisteriAdapter {
    fn service(&self) -> &str {
        self.op.service()
    }
    fn kind(&self) -> AdapterKind {
        if self.op.is_status() {
            AdapterKind::Pull
        } else {
            AdapterKind::Submit
        }
    }
    fn assurance(&self) -> DeliverySemantics {
        if self.op.is_status() {
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
        check_readiness(&self.profile, handle, Instant::now())?;
        let (record, expected, (path, action)) = self.record(envelope)?;
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
        interpret_status(body, &expected)
    }
}
/// The report version an accepted feedback item must carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionRule {
    Exactly(u32),
    AtLeast(u32),
}
/// What a matching, accepted processing-feedback item looks like for one record.
#[derive(Debug, Clone)]
pub struct ExpectedItem {
    pub delivery_data_type: &'static str,
    pub delivery_id: String,
    pub business_id: String,
    /// Payer's report reference the item must echo (new reports).
    pub item_id: Option<String>,
    /// Incomes Register report reference the item must carry (replacements).
    pub ir_item_id: Option<String>,
    pub version: VersionRule,
}
impl ExpectedItem {
    pub fn wage(report: &WageReport) -> Self {
        Self {
            delivery_data_type: DELIVERY_TYPE_WAGE,
            delivery_id: report.delivery_id(),
            business_id: report.business_id.as_str().into(),
            item_id: Some(report.report_id()),
            ir_item_id: None,
            version: VersionRule::Exactly(1),
        }
    }
    /// A replacement keeps the register's report reference and becomes the
    /// corrected version + 1. A cancellation answers the same way: the feedback
    /// item carries the report reference and the version of the "Cancelled" report
    /// the register generates, cancelled version + 1 (Data delivery – Schemas –
    /// Processing feedback, ValidItems).
    pub fn wage_correction(correction: &WageCorrection, timestamp: Instant) -> Self {
        Self {
            delivery_data_type: match correction.action {
                WageCorrectionAction::Replacement => DELIVERY_TYPE_WAGE,
                WageCorrectionAction::Cancellation => DELIVERY_TYPE_WAGE_CANCELLATION,
            },
            delivery_id: correction.delivery_id(timestamp),
            business_id: correction.report.business_id.as_str().into(),
            item_id: None,
            ir_item_id: Some(correction.ir_report_id.clone()),
            version: correction
                .corrects_version
                .map(|v| VersionRule::Exactly(v + 1))
                .unwrap_or(VersionRule::AtLeast(2)),
        }
    }
    /// A new report becomes version 1. A replacement keeps the register's report
    /// reference and gets the next version (the register assigns replaced + 1).
    pub fn employer(report: &EmployerSeparateReport, timestamp: Instant) -> Self {
        let (item_id, ir_item_id, version) = match &report.action {
            EmployerReportAction::New => (Some(report.report_id()), None, VersionRule::Exactly(1)),
            EmployerReportAction::Replacement {
                ir_report_id,
                replaces_version,
            } => (
                None,
                Some(ir_report_id.clone()),
                replaces_version
                    .map(|v| VersionRule::Exactly(v + 1))
                    .unwrap_or(VersionRule::AtLeast(2)),
            ),
        };
        Self {
            delivery_data_type: DELIVERY_TYPE_EMPLOYER,
            delivery_id: report.delivery_id(timestamp),
            business_id: report.business_id.as_str().into(),
            item_id,
            ir_item_id,
            version,
        }
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
pub fn interpret_status(
    xml: &str,
    expected: &ExpectedItem,
) -> Result<AdapterResponse, AdapterError> {
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
    if value(delivery, "DeliveryId") != Some(expected.delivery_id.as_str())
        || value(delivery, "DeliveryDataType") != Some(expected.delivery_data_type)
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
            || value(actor, "Code") != Some(expected.business_id.as_str())
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
            if items.len() != 1 || !items[0].has_tag_name("Item") {
                return Err(unknown("REPORT_IDENTITY"));
            }
            let item = items[0];
            let item_id = value(item, "ItemId");
            if expected.item_id.is_some() && item_id != expected.item_id.as_deref() {
                return Err(unknown("REPORT_IDENTITY"));
            }
            let version = value(item, "ItemVersion")
                .filter(|v| !v.starts_with('+') && !v.starts_with('0'))
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|v| match expected.version {
                    VersionRule::Exactly(n) => *v == n,
                    VersionRule::AtLeast(n) => *v >= n,
                })
                .ok_or_else(|| unknown("REPORT_IDENTITY"))?;
            let remote = value(item, "IRItemId")
                .filter(|s| ir_reference(s))
                .filter(|s| expected.ir_item_id.as_deref().is_none_or(|want| want == *s))
                .ok_or_else(|| unknown("REPORT_REFERENCE"))?;
            let delivery = value(status, "IRDeliveryId")
                .filter(|s| ir_reference(s))
                .ok_or_else(|| unknown("DELIVERY_REFERENCE"))?;
            Ok(AdapterResponse::Accepted {
                receipt: format!("{delivery}/{remote}/{version}"),
                body: json!({"delivery_id":expected.delivery_id,"report_id":item_id,"ir_delivery_id":delivery,"ir_report_id":remote,"version":version}),
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
    fn settings(from: &str) -> EmployerReportProfile {
        serde_json::from_value(json!({"registered_from":from,"contact_name":"Fictional Contact",
            "contact_phone":"+358 40 1234567","pension_insurances":[{"provider_code":46,"policy_number":"12-3456789"}]}))
        .unwrap()
    }

    #[test]
    fn employer_report_is_added_and_replaced_keeping_every_other_byte() {
        // Odd but valid layout: tabs, a trailing newline, keys in a custom order.
        let without = "{\n\t\"mode\": \"production\",\n\t\"schema_version\": 1,\n\t\"business_id\": \"1234567-1\",\n\t\"reporting_permission_active\": true,\n\t\"credentials_file\": null\n}\n";
        let added = with_employer_report(without, &settings("2020-02"))
            .unwrap()
            .text;
        let tail = "\n}\n";
        let head = &without[..without.len() - tail.len()];
        assert!(
            added.starts_with(&format!("{head},\n  \"employer_report\": {{")),
            "{added}"
        );
        assert!(added.ends_with(tail));
        assert_eq!(
            parse_wage_profile(added.as_bytes())
                .unwrap()
                .employer_report
                .unwrap()
                .registered_from,
            ReportMonth::parse("2020-02").unwrap()
        );

        let replaced = with_employer_report(&added, &settings("2021-03")).unwrap();
        assert_eq!(replaced.before.unwrap()["registered_from"], "2020-02");
        let replaced = replaced.text;
        let block = |s: &str| {
            let start = s.find("\"employer_report\"").unwrap();
            (
                s[..start].to_string(),
                s[s.rfind('}').unwrap()..].to_string(),
            )
        };
        assert_eq!(block(&added).0, block(&replaced).0);
        assert!(replaced.contains("\"registered_from\": \"2021-03\""));
        assert!(!replaced.contains("2020-02"));

        // Compact one-line JSON with the block first.
        let compact = r#"{"employer_report":{"registered_from":"2025-01","contact_name":"Old","contact_phone":"0401234567","pension_insurances":[]},"schema_version":1,"mode":"production","business_id":"1234567-1","reporting_permission_active":true,"credentials_file":null}"#;
        let out = with_employer_report(compact, &settings("2020-02"))
            .unwrap()
            .text;
        assert!(out.starts_with("{\"employer_report\":{\n"));
        assert!(out.ends_with(r#","schema_version":1,"mode":"production","business_id":"1234567-1","reporting_permission_active":true,"credentials_file":null}"#));
        assert!(with_employer_report("not json", &settings("2020-02")).is_err());
        // A bad hand-written block is repaired; a bad rest of the profile is not.
        let bad_block = compact.replace("\"Old\"", "\" Old \"");
        assert!(parse_wage_profile(bad_block.as_bytes()).is_err());
        let repaired = with_employer_report(&bad_block, &settings("2020-02")).unwrap();
        assert_eq!(repaired.before.unwrap()["contact_name"], " Old ");
        let bad_rest = compact.replace("\"1234567-1\"", "\"1234567-2\"");
        assert!(with_employer_report(&bad_rest, &settings("2020-02")).is_err());
        let duplicate = compact.replacen('{', "{\"employer_report\":null,", 1);
        assert!(with_employer_report(&duplicate, &settings("2020-02")).is_err());
    }

    #[test]
    fn employer_report_settings_validate_month_and_pension_insurances() {
        assert!(settings("2020-02").validate().is_ok());
        let mut s = settings("2020-02");
        s.pension_insurances[0].policy_number = "123456789012".into();
        assert!(s.validate().is_err());
        s.pension_insurances[0] = PensionInsurance {
            provider_code: 0,
            policy_number: "1".into(),
        };
        assert!(s.validate().is_err());
    }

    pub fn report() -> WageReport {
        serde_json::from_value(json!({"business_id":"1234567-1","employee_id":"fictional-employee","source_reference":"fictional-expense-2026-09","payment_date":"2026-10-25","period_start":"2026-09-01","period_end":"2026-09-30","joint_owner":true,"contact_name":"Fictional Contact","contact_phone":"+358 40 1234567","lines":[{"income_type":"311","amount":"120.00","kilometers":218},{"income_type":"331","amount":"54.00","allowance_code":3}]})).unwrap()
    }
    fn time() -> Instant {
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
            employer_report: None,
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
        for op in Op::ALL {
            let adapter = TulorekisteriAdapter {
                profile: profile(),
                op,
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
                interpret_status(&body, &ExpectedItem::wage(&report())),
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

        // Every unmet requirement is named, not one sentence for all of them.
        p = profile();
        p.mode = "disabled".into();
        p.reporting_permission_active = false;
        let message = check_readiness(&p, &h, time()).unwrap_err().to_string();
        assert!(message.contains("TULOREKISTERI_DISABLED"), "{message}");
        assert!(
            message.contains("TULOREKISTERI_REPORTING_PERMISSION_UNATTESTED"),
            "{message}"
        );
        assert!(!message.contains("CERTIFICATE"), "{message}");
    }
    #[test]
    fn readiness_is_itemised_with_owners() {
        let home = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let codes = |r: &TulorekisteriReadiness| -> Vec<&str> {
            r.blockers.iter().map(|b| b.code).collect()
        };

        let r = evaluate_readiness(home.path(), Ok(None), None, time());
        assert_eq!(codes(&r), ["TULOREKISTERI_PROFILE_MISSING"]);
        let r = evaluate_readiness(home.path(), Err("invalid".into()), None, time());
        assert_eq!(codes(&r), ["TULOREKISTERI_PROFILE_INVALID"]);

        let mut p = profile();
        p.reporting_permission_active = false;
        let r = evaluate_readiness(home.path(), Ok(Some(p.clone())), None, time());
        assert_eq!(
            codes(&r),
            [
                "TULOREKISTERI_REPORTING_PERMISSION_UNATTESTED",
                "TULOREKISTERI_CREDENTIAL_PATH_MISSING"
            ]
        );
        assert_eq!(r.blockers[0].owner, "human");
        assert_eq!(r.blockers[1].owner, "agent");

        let inside = home.path().join("credentials.json");
        std::fs::write(&inside, secret()).unwrap();
        let r = evaluate_readiness(home.path(), Ok(Some(profile())), Some(&inside), time());
        assert_eq!(codes(&r), ["TULOREKISTERI_CREDENTIAL_IN_ACCOUNTING_HOME"]);

        let credential = outside.path().join("credentials.json");
        let r = evaluate_readiness(home.path(), Ok(Some(profile())), Some(&credential), time());
        assert_eq!(codes(&r), ["TULOREKISTERI_CREDENTIAL_UNAVAILABLE"]);

        std::fs::write(&credential, secret()).unwrap();
        let r = evaluate_readiness(home.path(), Ok(Some(profile())), Some(&credential), time());
        assert!(r.ready, "{:?}", r.blockers);
        assert!(r.certificate.is_some());
        p = profile();
        p.business_id = BusinessId::parse("7654321-2").unwrap();
        let r = evaluate_readiness(home.path(), Ok(Some(p)), Some(&credential), time());
        assert_eq!(codes(&r), ["TULOREKISTERI_CREDENTIAL_BUSINESS_ID_MISMATCH"]);
        let later = "2040-01-01T00:00:00Z".parse().unwrap();
        let r = evaluate_readiness(home.path(), Ok(Some(profile())), Some(&credential), later);
        assert_eq!(codes(&r), ["TULOREKISTERI_CERTIFICATE_EXPIRED"]);

        // A key record signing cannot load is a blocker, not a send-time refusal.
        let mut bundle: serde_json::Value = serde_json::from_str(&secret()).unwrap();
        bundle["client_private_key_pem"] = json!(format!("\n{}", Pki::KEY));
        std::fs::write(&credential, bundle.to_string()).unwrap();
        let r = evaluate_readiness(home.path(), Ok(Some(profile())), Some(&credential), time());
        assert_eq!(codes(&r), ["TULOREKISTERI_CREDENTIAL_INVALID"]);
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
            (
                "status",
                status_record(
                    DELIVERY_TYPE_WAGE,
                    &r.report.delivery_id(),
                    r.report.business_id.as_str(),
                    r.timestamp,
                ),
            ),
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
    fn correction(action: WageCorrectionAction) -> WageCorrectionRequest {
        WageCorrectionRequest {
            correction: WageCorrection {
                action,
                report: report(),
                ir_report_id: "22222222222233334444555555555555".into(),
                corrects_version: Some(1),
            },
            hetu: match action {
                WageCorrectionAction::Replacement => "010101A123N".into(),
                WageCorrectionAction::Cancellation => String::new(),
            },
            timestamp: "2026-11-02T08:00:00Z".parse().unwrap(),
            status_timestamp: None,
        }
    }
    fn correction_feedback(r: &WageCorrectionRequest, item: &str) -> String {
        let data_type = match r.correction.action {
            WageCorrectionAction::Replacement => "100",
            WageCorrectionAction::Cancellation => "105",
        };
        feedback(
            "3",
            &format!("<ValidItems><Item>{item}</Item></ValidItems>"),
        )
        .replace(
            &report().delivery_id(),
            &r.correction.delivery_id(r.timestamp),
        )
        .replace(
            "<DeliveryDataType>100<",
            &format!("<DeliveryDataType>{data_type}<"),
        )
    }
    const CORRECTED_ITEM: &str = "<ItemId>r-any</ItemId><IRItemId>22222222222233334444555555555555</IRItemId><ItemVersion>2</ItemVersion>";
    #[test]
    fn wage_correction_sends_replacement_or_cancellation_once_and_reads_status() {
        for (action, path, soap_action, ns) in [
            (
                WageCorrectionAction::Replacement,
                "WageReportService.svc",
                "\"SendWageReport\"",
                WAGE_NS,
            ),
            (
                WageCorrectionAction::Cancellation,
                "InvalidationService.svc",
                "\"SendInvalidation\"",
                crate::tulorekisteri_xml::INVALIDATION_NS,
            ),
        ] {
            let mut r = correction(action);
            let mut s = Scenario::file_receipt("wage-soap");
            s.soap = Some(SoapScenario {
                send_body: correction_feedback(&r, CORRECTED_ITEM),
                send_status: 200,
                send_delay: Duration::ZERO,
                status_body: correction_feedback(&r, CORRECTED_ITEM),
            });
            let m = MockVeroVatServer::start_wage_mtls(s).unwrap();
            let h = tempfile::tempdir().unwrap();
            let b = broker(h.path(), &m, Duration::from_secs(2));
            let sent = b
                .submit(
                    "test",
                    "effect",
                    &wage_correction_envelope(&r, false).unwrap(),
                )
                .unwrap();
            assert_eq!(
                sent,
                RemoteOutcome::Delivered {
                    receipt: "11111111222233334444555555555555/22222222222233334444555555555555/2"
                        .into()
                }
            );
            // A status query carries no personal identity code.
            r.hetu = String::new();
            r.status_timestamp = Some("2026-11-02T09:00:00Z".parse().unwrap());
            let status = b
                .fetch(&wage_correction_envelope(&r, true).unwrap())
                .unwrap();
            assert_eq!(status["version"], 2);
            let requests = m.requests();
            assert_eq!(requests.len(), 2);
            assert!(requests[0].path.ends_with(path));
            assert_eq!(requests[0].headers["soapaction"], soap_action);
            let body = std::str::from_utf8(&requests[0].body).unwrap();
            assert!(body.contains(ns) && body.contains("<ds:Signature"));
            assert_eq!(
                body.contains("010101A123N"),
                action == WageCorrectionAction::Replacement
            );
            assert!(requests[1].path.ends_with("StatusService.svc"));
            let status_body = std::str::from_utf8(&requests[1].body).unwrap();
            assert!(status_body.contains(&format!(
                "<DeliveryDataType>{}</DeliveryDataType>",
                if action == WageCorrectionAction::Replacement {
                    "100"
                } else {
                    "105"
                }
            )));
            let outbox = std::fs::read(h.path().join("integration/outbox/outbox.db")).unwrap();
            assert!(!String::from_utf8_lossy(&outbox).contains("010101A123N"));
        }
    }
    #[test]
    fn wage_correction_feedback_must_name_the_corrected_report_and_next_version() {
        for action in [
            WageCorrectionAction::Replacement,
            WageCorrectionAction::Cancellation,
        ] {
            let r = correction(action);
            let expected = ExpectedItem::wage_correction(&r.correction, r.timestamp);
            assert!(interpret_status(&correction_feedback(&r, CORRECTED_ITEM), &expected).is_ok());
            for bad in [
                // The version being corrected, not the corrected one.
                CORRECTED_ITEM.replace("<ItemVersion>2", "<ItemVersion>1"),
                // Another report.
                CORRECTED_ITEM.replace("2222222222223333", "9999999999993333"),
            ] {
                assert!(matches!(
                    interpret_status(&correction_feedback(&r, &bad), &expected),
                    Err(AdapterError::AmbiguousDelivery { .. })
                ));
            }
            // Feedback of the other record type is not this correction's answer.
            let other = correction(match action {
                WageCorrectionAction::Replacement => WageCorrectionAction::Cancellation,
                WageCorrectionAction::Cancellation => WageCorrectionAction::Replacement,
            });
            assert!(
                interpret_status(&correction_feedback(&other, CORRECTED_ITEM), &expected).is_err()
            );
            // Without the corrected version any later version is the answer.
            let mut unversioned = correction(action);
            unversioned.correction.corrects_version = None;
            let expected =
                ExpectedItem::wage_correction(&unversioned.correction, unversioned.timestamp);
            let later = CORRECTED_ITEM.replace("<ItemVersion>2", "<ItemVersion>3");
            assert!(
                interpret_status(&correction_feedback(&unversioned, &later), &expected).is_ok()
            );
        }
    }
    #[test]
    fn wage_correction_adapter_refuses_mismatched_requests() {
        let m = mock(accepted(), 200, Duration::ZERO);
        let h = tempfile::tempdir().unwrap();
        let b = broker(h.path(), &m, Duration::from_secs(2));
        // A send must not carry a status time, and a replacement needs the HETU.
        let mut r = correction(WageCorrectionAction::Cancellation);
        r.status_timestamp = Some(r.timestamp);
        let mut no_hetu = correction(WageCorrectionAction::Replacement);
        no_hetu.hetu = String::new();
        let mut other_payer = correction(WageCorrectionAction::Cancellation);
        other_payer.correction.report.business_id = BusinessId::parse("7654321-2").unwrap();
        for (i, bad) in [r, no_hetu, other_payer].iter().enumerate() {
            let outcome = b
                .submit(
                    "test",
                    &format!("effect-{i}"),
                    &wage_correction_envelope(bad, false).unwrap(),
                )
                .unwrap();
            assert!(
                matches!(outcome, RemoteOutcome::Rejected { .. }),
                "{outcome:?}"
            );
        }
        assert!(m.requests().is_empty());
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

    fn employer_request(action: EmployerReportAction) -> EmployerReportRequest {
        EmployerReportRequest {
            report: serde_json::from_value(json!({"business_id":"1234567-1","month":"2026-10","content":{"kind":"health_insurance_contribution","total":"0.00"},"action":action,"contact_name":"Fictional Contact","contact_phone":"+358 40 1234567","pension_insurances":[]})).unwrap(),
            timestamp: "2026-11-02T08:00:00Z".parse().unwrap(),
            status_timestamp: None,
        }
    }
    fn employer_feedback(r: &EmployerReportRequest, status: &str, item: &str) -> String {
        let actors = [
            "DeliveryDataOwner",
            "DeliveryDataCreator",
            "DeliveryDataSender",
        ]
        .iter()
        .map(|n| format!("<{n}><Type>1</Type><Code>1234567-1</Code></{n}>"))
        .collect::<String>();
        soap(&format!("<ir:StatusResponseFromIR xmlns:ir=\"{RESPONSE_NS}\"><DeliveryData><DeliveryDataType>101</DeliveryDataType><DeliveryId>{}</DeliveryId><ProductionEnvironment>true</ProductionEnvironment>{actors}</DeliveryData><StatusResponse><DeliveryDataStatus>{status}</DeliveryDataStatus><IRDeliveryId>33333333222233334444555555555555</IRDeliveryId>{item}</StatusResponse></ir:StatusResponseFromIR>", r.report.delivery_id(r.timestamp)))
    }
    fn employer_item(item_id: &str, ir: &str, version: u32) -> String {
        format!("<ValidItems><Item><ItemId>{item_id}</ItemId><IRItemId>{ir}</IRItemId><ItemVersion>{version}</ItemVersion></Item></ValidItems>")
    }
    /// Requests and records carry instants in two fixed forms: the request JSON
    /// in serde form (`Z`) and the XML `<Timestamp>` in `+00:00` form, both with
    /// 0, 3, 6 or 9 fractional digits. Pinned so that the delivery reference and
    /// the bytes a preview approved stay reproducible.
    #[test]
    fn requests_and_records_write_instants_as_before() {
        let mut wage = request();
        wage.timestamp = "2026-10-25T12:00:00.120Z".parse().unwrap();
        let value = serde_json::to_value(&wage).unwrap();
        assert_eq!(value["timestamp"], "2026-10-25T12:00:00.120Z");
        let back: WageRequest = serde_json::from_value(value).unwrap();
        assert_eq!(back.timestamp, wage.timestamp);
        let record = wage_record(&wage.report, &wage.hetu, wage.timestamp).unwrap();
        assert!(
            record.contains("<Timestamp>2026-10-25T12:00:00.120+00:00</Timestamp>"),
            "{record}"
        );

        let mut employer = employer_request(EmployerReportAction::New);
        employer.timestamp = "2026-11-02T08:00:00.000001Z".parse().unwrap();
        employer.status_timestamp = Some("2026-11-03T09:30:00.123456789+00:00".parse().unwrap());
        let value = serde_json::to_value(&employer).unwrap();
        assert_eq!(value["timestamp"], "2026-11-02T08:00:00.000001Z");
        assert_eq!(value["status_timestamp"], "2026-11-03T09:30:00.123456789Z");
        let back: EmployerReportRequest = serde_json::from_value(value).unwrap();
        assert_eq!(back.timestamp, employer.timestamp);
        assert_eq!(back.status_timestamp, employer.status_timestamp);
        let record = payer_summary_record(&employer.report, employer.timestamp).unwrap();
        assert!(
            record.contains("<Timestamp>2026-11-02T08:00:00.000001+00:00</Timestamp>"),
            "{record}"
        );
        assert!(
            record.contains("<ReportDate>2026-11-02</ReportDate>"),
            "{record}"
        );
        assert!(record.contains("<DeliveryId>er-202610-"), "{record}");
        let status = crate::tulorekisteri_xml::status_record(
            "101",
            "er-202610-x",
            "1234567-1",
            employer.status_timestamp.unwrap(),
        );
        assert!(
            status.contains("<Timestamp>2026-11-03T09:30:00.123456789+00:00</Timestamp>"),
            "{status}"
        );
        let whole = crate::tulorekisteri_xml::status_record(
            "101",
            "er-202610-x",
            "1234567-1",
            "2026-11-03T09:30:00Z".parse().unwrap(),
        );
        assert!(
            whole.contains("<Timestamp>2026-11-03T09:30:00+00:00</Timestamp>"),
            "{whole}"
        );
    }
    #[test]
    fn certificate_validity_is_written_as_before() {
        let meta = VeroCredentials::parse(&secret())
            .unwrap()
            .validate_transport_material()
            .unwrap();
        let value = serde_json::to_value(&meta).unwrap();
        assert_eq!(value["not_before"], "2026-10-24T19:13:18Z");
        assert_eq!(value["not_after"], "2036-10-21T19:13:18Z");
    }
    const IR_REPORT: &str = "44444444222233334444555555555555";
    #[test]
    fn employer_report_mtls_send_and_status_use_the_payer_summary_service() {
        let r = employer_request(EmployerReportAction::New);
        let ok = employer_feedback(&r, "3", &employer_item("agg-er-202610", IR_REPORT, 1));
        let mut s = Scenario::file_receipt("wage-soap");
        s.soap = Some(SoapScenario {
            send_body: ok.clone(),
            send_status: 200,
            send_delay: Duration::ZERO,
            status_body: ok,
        });
        let m = MockVeroVatServer::start_wage_mtls(s).unwrap();
        let h = tempfile::tempdir().unwrap();
        let b = broker(h.path(), &m, Duration::from_secs(2));
        let outcome = b
            .submit("test", "effect", &employer_envelope(&r, false).unwrap())
            .unwrap();
        assert_eq!(
            outcome,
            RemoteOutcome::Delivered {
                receipt: format!("33333333222233334444555555555555/{IR_REPORT}/1")
            }
        );
        let status = EmployerReportRequest {
            status_timestamp: Some("2026-11-02T09:00:00Z".parse().unwrap()),
            ..employer_request(EmployerReportAction::New)
        };
        let body = b.fetch(&employer_envelope(&status, true).unwrap()).unwrap();
        assert_eq!(body["ir_report_id"], IR_REPORT);
        let requests = m.requests();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].path.ends_with("/PayerSummaryReportService.svc"));
        assert_eq!(
            requests[0].headers["soapaction"],
            "\"SendPayerSummaryReport\""
        );
        assert!(requests[1].path.ends_with("/StatusService.svc"));
        let sent = std::str::from_utf8(&requests[0].body).unwrap();
        assert!(sent.contains(crate::tulorekisteri_xml::PSR_NS) && sent.contains("<ds:Signature"));
        let query = std::str::from_utf8(&requests[1].body).unwrap();
        assert!(query.contains("<DeliveryDataType>101</DeliveryDataType>"));
        assert!(query.contains(&r.report.delivery_id(r.timestamp)));
        // A status envelope without a query time, or a send envelope with one, is refused offline.
        assert!(b.fetch(&employer_envelope(&r, true).unwrap()).is_err());
    }
    #[test]
    fn employer_feedback_must_match_action_and_version() {
        let new = employer_request(EmployerReportAction::New);
        let exp = |r: &EmployerReportRequest| ExpectedItem::employer(&r.report, r.timestamp);
        let accepted = |r: &EmployerReportRequest, item: &str| {
            interpret_status(&employer_feedback(r, "3", item), &exp(r))
        };
        assert!(accepted(&new, &employer_item("agg-er-202610", IR_REPORT, 1)).is_ok());
        for item in [
            employer_item("agg-er-202609", IR_REPORT, 1),
            employer_item("agg-er-202610", IR_REPORT, 2),
            employer_item("agg-er-202610", "not-a-reference", 1),
        ] {
            assert!(matches!(
                accepted(&new, &item),
                Err(AdapterError::AmbiguousDelivery { .. })
            ));
        }
        // Wage feedback (record type 100) never proves an employer report.
        let wage_type = employer_feedback(&new, "3", &employer_item("agg-er-202610", IR_REPORT, 1))
            .replace("<DeliveryDataType>101", "<DeliveryDataType>100");
        assert!(interpret_status(&wage_type, &exp(&new)).is_err());
        let known = employer_request(EmployerReportAction::Replacement {
            ir_report_id: IR_REPORT.into(),
            replaces_version: Some(1),
        });
        assert!(accepted(&known, &employer_item("agg-er-202610", IR_REPORT, 2)).is_ok());
        assert!(accepted(&known, &employer_item("agg-er-202610", IR_REPORT, 3)).is_err());
        assert!(accepted(
            &known,
            &employer_item("agg-er-202610", "55555555222233334444555555555555", 2)
        )
        .is_err());
        let unknown_version = employer_request(EmployerReportAction::Replacement {
            ir_report_id: IR_REPORT.into(),
            replaces_version: None,
        });
        assert!(accepted(&unknown_version, &employer_item("other-ref", IR_REPORT, 4)).is_ok());
        assert!(accepted(&unknown_version, &employer_item("other-ref", IR_REPORT, 1)).is_err());
        assert!(matches!(
            interpret_status(&employer_feedback(&new, "5", ""), &exp(&new)),
            Err(AdapterError::Rejected { .. })
        ));
        assert!(matches!(
            interpret_status(&employer_feedback(&new, "2", ""), &exp(&new)),
            Err(AdapterError::AmbiguousDelivery { .. })
        ));
    }
    #[test]
    fn golden_signed_employer_record() {
        let r = employer_request(EmployerReportAction::New);
        let signed = sign_record(
            &payer_summary_record(&r.report, r.timestamp).unwrap(),
            &handle(),
        )
        .unwrap();
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/employer-report-signed.xml");
        if std::env::var_os("UPDATE_WAGE_GOLDEN").is_some() {
            std::fs::write(&path, &signed).unwrap();
        }
        assert_eq!(signed, std::fs::read_to_string(path).unwrap());
    }
}
