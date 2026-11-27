//! `aggountant` — CLI for the A1 sealed-voucher light path (F15).
//!
//! Primitives only (`candidate.md §8`): `chart list`, `voucher draft/validate/
//! seal/verify`, `artifact attach`. Output rules follow `AGENTS-AI-FIRST-CLI.md`:
//! `--json` structured output, data to stdout, machine-readable errors to
//! stderr, exit codes 0 (ok) / 1 (user error) / 2 (system error). No
//! interactive prompts; structured input arrives as JSON on stdin.

use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::{error::ErrorKind, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use aggountant_broker::{
    evaluate_vero_readiness, read_vero_profile, write_vero_profile, Broker, ConsultLlmAdapter,
    EmailPullAdapter, KauppatiliPullAdapter, PankkiMaksuAdapter, RawResponse, Registry,
    RemoteOutcome, ScriptedTransport, ScriptedVeroFileVatReturnAdapter,
    ScriptedVeroFiledVatReturnAdapter, ScriptedVeroVatPeriodsAdapter, SyntheticSubmitAdapter,
    Transport, TransportError, TulorekisteriAdapter, Vault, VeroCredentialTarget,
    VeroFileVatReturnAdapter, VeroFiledVatReturnAdapter, VeroFilingMode, VeroFilingProfile,
    VeroReadinessPurpose, VeroVatPeriodsAdapter, VeroVerotiliAdapter, SVC_CONSULT_LLM,
    SVC_EMAIL_PULL, SVC_KAUPPATILI_PULL, SVC_PANKKI_MAKSU, SVC_SYNTHETIC_SUBMIT, SVC_TULOREKISTERI,
    SVC_VERO_FILED_VAT_RETURN, SVC_VERO_FILE_VAT_RETURN, SVC_VERO_VAT_PERIODS, SVC_VERO_VEROTILI,
};
use aggountant_domain::{build_draft, DraftInput, PayrollIntent, PolicySet};
use aggountant_import::{
    parse_camt, project_voucher_with_role_table, Assignment, ExistingVoucher, ImportError,
};
use aggountant_ledger::{EmployeeState, Ledger, SealRequest, VoucherListQuery};
use aggountant_model::{
    BusinessId, Date, FilingContactName, FilingContactPhone, Hash, Money, PeriodId, VoucherDraft,
};
use aggountant_process::{
    prepare_structured_receipt, record_structured_receipt, CaptureReceiptInput, CloseYearInput,
    CloseYearNextFor, FileVatInput, FileVatStatusPlan, HuntReceiptsInput, ProcessEngine, RunView,
    StructuredReceiptInput, SystemClock, VatReconciliationAcknowledgement,
};

/// The Vero transport note, a macro so `input_help` can `concat!` it into the
/// `file-vat start` INPUT section.
macro_rules! vero_transport_help {
    () => {
        "Vero transport: Broker reliability uses an outbox and OutcomeUnknown reconciliation. Production uses the fixed Vero.fi target only after `aggountant vero-fi status` reports ready. The credential bundle path is platform-local (AGGOUNTANT_VERO_CREDENTIALS_FILE is only a path override). AGGOUNTANT_BROKER_SCRIPT selects the scripted test seam only and never establishes production readiness; output (and errors) of a command that used it carry a `test_seam` field, and without --json a warning goes to stderr."
    };
}

mod bank_import;
mod claim;
mod explain;
mod home;
mod input_help;
mod knowledge;
mod reports;
mod skill;
mod tulorekisteri;
mod vero_fi;
mod wage;

use home::HomeCmd;
use knowledge::KnowledgeCmd;
use reports::ReportCmd;
use vero_fi::resolve_credential_path;

const VERO_TRANSPORT_HELP: &str = vero_transport_help!();

#[derive(Parser, Debug)]
#[command(
    name = "aggountant",
    version,
    about = "AI-first Finnish bookkeeping — S1 sealed-voucher light path"
)]
struct Cli {
    /// Data home (fiscal + masterdata + artifacts). Precedence: --home,
    /// $AGGOUNTANT_HOME, config-file `home`, nearest .aggountant marker, ./accounting.
    /// Only `init` creates a home; other commands refuse a path that holds none
    /// (HOME_NOT_INITIALIZED, naming the path and how it was chosen).
    #[arg(long, global = true)]
    home: Option<PathBuf>,
    /// Emit structured JSON (default output is a concise human line).
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Tulorekisteri: tax-free travel earnings report (offline preview/start, explicit approval, reconcile-only recovery), its corrections (`file-wage correction`: replacement or cancellation) and the monthly employer's separate report (`file-wage employer-report`).
    FileWage {
        #[command(subcommand)]
        cmd: wage::WageCmd,
    },
    /// Expense claims: `claim compute` previews the scheduled tax-free travel
    /// allowances of trips from stdin JSON (read-only; no company home needed).
    Claim {
        #[command(subcommand)]
        cmd: claim::ClaimCmd,
    },
    /// Inspect effective configuration and data-root resolution.
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
    /// Explain an error code from the user guide's error-code reference, bundled
    /// with this binary (offline): what happened and what to do. Without CODE,
    /// list the codes the guide explains. Error JSON already carries the
    /// guide's Finnish "what to do" as `hint` for these codes.
    Explain {
        /// The `error_code` of a failed command, e.g. PERIOD_SEALED.
        code: Option<String>,
        /// Language of the guide entry.
        #[arg(long, value_enum, default_value = "fi")]
        lang: explain::Lang,
    },
    /// List, install, or print the companion AI operating manuals bundled with this binary.
    Skill {
        #[command(subcommand)]
        cmd: SkillCmd,
    },
    /// Bootstrap a company home: write the git-native reference masterdata
    /// (chart of accounts + account kinds + content-hashed chart_version) as
    /// diffable plaintext. Safe to re-run — refuses to clobber an existing home.
    Init,
    /// Maintain an existing company home: `home upgrade` after updating aggountant.
    Home {
        #[command(subcommand)]
        cmd: HomeCmd,
    },
    /// Chart of accounts (masterdata read).
    Chart {
        #[command(subcommand)]
        cmd: ChartCmd,
    },
    /// Voucher primitives.
    Voucher {
        #[command(subcommand)]
        cmd: VoucherCmd,
    },
    /// Artifact blob store.
    Artifact {
        #[command(subcommand)]
        cmd: ArtifactCmd,
    },
    /// Fiscal period state (ADR-6): show state, or mark a sealed period's VAT
    /// return filed (thin `file-vat` stand-in for the reopen guard).
    Period {
        #[command(subcommand)]
        cmd: PeriodCmd,
    },
    /// Workflow Runs (F10 process engine): the `close-period` Run and its drivers.
    Run {
        #[command(subcommand)]
        cmd: RunCmd,
    },
    /// Inspect and configure the own-organisation Incomes Register (Tulorekisteri)
    /// channel profile: `tulorekisteri status` (offline readiness, itemised) and
    /// `tulorekisteri configure`.
    Tulorekisteri {
        #[command(subcommand)]
        cmd: tulorekisteri::TulorekisteriCmd,
    },
    /// Inspect and configure the own-organisation Vero.fi filing boundary.
    VeroFi {
        #[command(subcommand)]
        cmd: VeroFiCmd,
    },
    /// The `file-vat` Run (RF3/S3): compute + submit the VAT return and payment via
    /// the Broker edge substrate (outbox / OutcomeUnknown reconciliation). Direct Vero
    /// transport uses the fixed production target plus readiness-approved local profile and
    /// credential inputs. `$AGGOUNTANT_BROKER_SCRIPT` is a scripted test seam only.
    #[command(after_long_help = VERO_TRANSPORT_HELP)]
    FileVat {
        #[command(subcommand)]
        cmd: FileVatCmd,
    },
    /// F7 employee master-data (separate gitignored PII store). Add/inspect the
    /// employee records the `run-payroll` Run reads.
    Employee {
        #[command(subcommand)]
        cmd: EmployeeCmd,
    },
    /// The `run-payroll` Run (RF4/S4): compute the batch, approve it, and drive each
    /// employee's payment + Tulorekisteri reports over the Broker edge. The authority
    /// transport is supplied by `$AGGOUNTANT_BROKER_SCRIPT` (as for `file-vat`).
    RunPayroll {
        #[command(subcommand)]
        cmd: RunPayrollCmd,
    },
    /// Capture a supported ordinary receipt with one stateless approval, or escalate a
    /// non-routine receipt through the existing consult Run.
    CaptureReceipt {
        #[command(subcommand)]
        cmd: CaptureReceiptCmd,
    },
    /// The `hunt-receipts` Run (RF2/S5): hunt evidence for missing-evidence bank
    /// transactions via targeted email/kauppatili pulls, park on `Effect::Wait` under a
    /// quota backoff, and seal each match on its own approval.
    HuntReceipts {
        #[command(subcommand)]
        cmd: HuntReceiptsCmd,
    },
    /// F13 obligations: list open/resolved obligations, or resolve one (unblocking
    /// `close-period` for its period).
    Obligation {
        #[command(subcommand)]
        cmd: ObligationCmd,
    },
    /// Read consult knowledge and maintain company-local facts and practices.
    Knowledge {
        #[command(subcommand)]
        cmd: KnowledgeCmd,
    },
    /// The RF2 intake stage: list the typed match candidates a hunt produced.
    Intake {
        #[command(subcommand)]
        cmd: IntakeCmd,
    },
    /// Year-end close (tilinpäätös) of a fiscal year whose months are all sealed:
    /// depreciation (poistot), accruals (jaksotukset) and corrections, four approval
    /// gates, and finally sealing the fiscal year. Every command returns `message`
    /// and `next_command`, which name the open gate and what approving it seals.
    CloseYear {
        #[command(subcommand)]
        cmd: CloseYearCmd,
    },
    /// Read-only financial reports from sealed ledger data.
    Report {
        #[command(subcommand)]
        cmd: ReportCmd,
    },
    /// Fiscal years: define a fiscal year that is not the calendar year, list the
    /// definitions, and show a year's dates and authoritative `Open`/`Sealed` state.
    FiscalYear {
        #[command(subcommand)]
        cmd: FiscalYearCmd,
    },
    /// Import source files into vouchers (F23 MVP). `import camt …` parses a
    /// Holvi bank statement — camt.053 **or** camt.052 (Holvi's self-service
    /// "Electronic account statement" export gives .052) — into voucher intents
    /// and (given the skill's account assignments) seals them via the existing
    /// record→seal path.
    Import {
        #[command(subcommand)]
        cmd: ImportCmd,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCmd {
    /// Print the platform-default configuration file path without reading it.
    Path,
    /// Show effective configuration values and their source provenance. The optional
    /// JSON file accepts `{"home":"/path/to/accounting"}`; unknown keys are rejected.
    Show,
}

#[derive(Subcommand, Debug)]
enum VeroFiCmd {
    /// Inspect local production readiness without making a network request.
    Status,
    /// Merge non-secret filing facts into the stored profile and store a platform-local credential path.
    #[command(after_help = input_help::VERO_FI_CONFIGURE)]
    Configure {
        /// Validate and report the changes and the resulting readiness without writing either file.
        #[arg(long)]
        dry_run: bool,
        /// Write the input as the whole profile instead of merging it; drops any stored field the input leaves out, including verification evidence.
        #[arg(long)]
        replace: bool,
    },
    /// Perform an explicitly requested safe test-environment verification read.
    Verify {
        /// Only `test` is accepted; file-vat never uses this target.
        #[arg(long, value_enum)]
        target: VeroVerifyTarget,
    },
    /// Run the three fixed API Portal integration-verification scenarios.
    ///
    /// Uses production credentials with Vero's static Verification endpoints. It
    /// does not file taxes, change the disabled|production mode, or accept a URL.
    VerifyPortal,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum VeroVerifyTarget {
    Test,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CliFileVatInput {
    period: PeriodId,
    sales_vat: Money,
    purchase_vat: Money,
    #[serde(default)]
    business_id: Option<BusinessId>,
    #[serde(default)]
    contact_name: Option<FilingContactName>,
    #[serde(default)]
    contact_phone: Option<FilingContactPhone>,
    payee_iban: String,
    #[serde(default)]
    vat_reconciliation_acknowledgement: Option<VatReconciliationAcknowledgement>,
    #[serde(default)]
    replacement_reasons: Option<Vec<aggountant_broker::VatReplacementReason>>,
}

#[derive(Subcommand, Debug)]
enum SkillCmd {
    /// List the companion skills bundled with this binary.
    List,
    /// Install one skill, or all bundled skills when NAME is omitted.
    /// A named skill brings the skills it links to: every skill installs
    /// aggountant-init (the shared SOUL.md), and vero-fi and tulorekisteri
    /// install each other and e-services.
    Install {
        /// Skill name; omit to install every bundled skill.
        name: Option<String>,
        /// Agent-runtime skill root, replacing the defaults (default: ~/.claude/skills).
        #[arg(long)]
        target: Option<PathBuf>,
    },
    /// Print one canonical bundled SKILL.md to stdout without side effects.
    /// `--file` prints another file the skill ships or links to instead, for
    /// example the shared role: `skill print aggountant-init --file SOUL.md`.
    Print {
        /// Skill name from `skill list`.
        name: String,
        /// A file as the skill's text links it: one of the skill's own files
        /// (`skill list --json` lists them) or a required skill's file as
        /// `../<skill>/<file>` (default: SKILL.md).
        #[arg(long)]
        file: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ImportCmd {
    /// camt.053 / camt.052 (ISO 20022 bank statement) — the Holvi bank leg.
    /// camt.052 (`BkToCstmrAcctRpt`) is what Holvi's self-service "Electronic
    /// account statement" export produces. `camt053` and `camt052` are kept as
    /// aliases for callers written before the verb was renamed.
    #[command(visible_aliases = ["camt053", "camt052"])]
    Camt {
        #[command(subcommand)]
        cmd: CamtCmd,
    },
}

#[derive(Subcommand, Debug)]
enum CamtCmd {
    /// Parse a camt.053 or camt.052 file into voucher intents (deterministic;
    /// **no** account or VAT assignment — that is the skill's judgment). Emits
    /// the canonical bank-transaction rows + their stable dedup keys, and under
    /// `statements` each statement's period, opening and closing balances and
    /// whether its booked entries carry the one to the other
    /// (`balances_reconcile`). A report with no booked entries is not an
    /// error: the output then carries `empty_report` with the account, the
    /// period and a message to check before treating the month as quiet.
    Parse {
        #[arg(long)]
        file: PathBuf,
    },
    /// Seal parsed intents into the ledger via the existing record→seal path.
    /// Reads per-entry assignments from stdin JSON. Contra amounts are gross:
    /// non-zero-rate legacy `vat_code` tags are refused. Purchase `vat_treatment`
    /// requires that assignment's explicit `document_date` (YYYY-MM-DD); it expands
    /// VAT-A legs using that source date while the bank date remains the posting
    /// date. Untagged/zero-rate imports may omit it. Sales VAT expansion is
    /// unsupported here; do not import such an assignment. The original XML is
    /// retained once (KPL 2:5). A transaction already in the books from another
    /// export (same bank reference and account) is not booked again when its bank
    /// facts and its booking are unchanged; otherwise the batch stops
    /// (SOURCE_FACTS_CHANGED, POSTING_PROJECTION_CHANGED). The batch is written
    /// whole or not at all. First run with `--dry-run`: it returns every
    /// voucher it would seal and a `preparation_hash`, writing nothing; after the
    /// user approves, seal with `--approved-preparation-hash`.
    #[command(after_help = input_help::IMPORT_CAMT_SEAL)]
    Seal {
        #[arg(long)]
        file: PathBuf,
        /// Prepare and validate every voucher and return them with the
        /// preparation hash; write nothing.
        #[arg(long, conflicts_with = "approved_preparation_hash")]
        dry_run: bool,
        /// The `preparation_hash` of the approved `--dry-run`. The seal is
        /// refused (`IMPORT_PREPARATION_CHANGED`) unless it prepares exactly
        /// those vouchers.
        #[arg(long)]
        approved_preparation_hash: Option<String>,
    },
    /// List which bank transactions each retained statement file contained:
    /// one row per file and transaction, with the voucher that books it. A
    /// transaction seen in several overlapping exports has a row for each.
    Observations {
        /// Only the statement file with this content address (the
        /// `attachment_ref` of a seal).
        #[arg(long)]
        attachment_ref: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum CloseYearCmd {
    /// Start a year-end close from stdin JSON (year, assets, accruals, corrections).
    /// Puts nothing in the books: it computes the depreciation and stops at the first
    /// gate, `ApproveAdjustments`. Assets of the previous sealed year are carried
    /// from its register at their closing book value. `--dry-run` shows the same
    /// computation without starting a run.
    #[command(after_help = input_help::CLOSE_YEAR_START)]
    Start {
        /// Compute and show the assets, their depreciation method, rate and amount,
        /// and what would block the start, without creating a run or writing
        /// anything. Takes the same stdin JSON; `{"year": N}` alone shows the
        /// carried assets.
        #[arg(long)]
        dry_run: bool,
    },
    /// Answer the run's open gate with an explicit `--answer approve` or
    /// `--answer reject`; there is no default. The gates open in this order: `ApproveAdjustments` (approving
    /// seals the depreciation and accrual vouchers into the books, permanently: a
    /// sealed voucher is corrected only by a new one), `ApproveTaseErittelyt` (the
    /// balance-sheet breakdowns) and `ApproveLiitetiedot` (the notes), whose
    /// approvals seal nothing, and `ApproveStatements` (approving seals the fiscal
    /// year; this cannot be undone). Read the gate with `close-year show` first.
    /// `reject` declines the close and ends the run; what it already sealed stays in
    /// the books. Start a new run after fixing the input.
    Answer {
        /// The run to answer, as returned by `close-year start` (`cy-…`).
        #[arg(long)]
        run_id: String,
        /// The open gate: ApproveAdjustments, ApproveTaseErittelyt,
        /// ApproveLiitetiedot or ApproveStatements.
        #[arg(long)]
        gate: String,
        /// `approve` or `reject`. Required: approving ApproveStatements seals the
        /// fiscal year irreversibly, so no answer is ever assumed.
        #[arg(long)]
        answer: String,
    },
    /// Seal one correction given in the start input into the books. A correction is
    /// a balanced entry for the closing year that the program does not calculate: a
    /// reclassification, an error found in the closed months, or a depreciation
    /// calculated outside the program. It is permanent once sealed (corrected only by
    /// a new entry), so seal it only after the user has approved it. Every correction
    /// is sealed while `ApproveAdjustments` is open, before that gate is approved.
    SealCorrection {
        /// The run whose correction is sealed (`cy-…`).
        #[arg(long)]
        run_id: String,
        /// The correction's `correction_id` from the `close-year start` input.
        #[arg(long)]
        correction: String,
    },
    /// Continue an interrupted run: finish what the user already approved, including
    /// sealing the fiscal year when `ApproveStatements` was approved, and stop at the
    /// open gate. Also rewrites `archive/<year>/tilinpaatos.json` when the year this
    /// run sealed has no archive. Not a read; use `show` to look.
    Resume {
        /// The run to resume (`cy-…`).
        #[arg(long)]
        run_id: String,
    },
    /// Show the run and the content of its open gate (a read; drives nothing):
    /// depreciation and accruals, tase-erittelyt, liitetiedot or the statements,
    /// exactly as the gate's `snapshot_hash` binds them.
    Show {
        /// The run to show (`cy-…`).
        #[arg(long)]
        run_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum FiscalYearCmd {
    /// Show a fiscal year's dates and authoritative state (`Open`/`Sealed`). A
    /// fiscal year is identified by the calendar year it ends in (`--year`), or
    /// found by a date inside it (`--date YYYY-MM-DD`).
    Show {
        #[arg(long, required_unless_present = "date", conflicts_with = "date")]
        year: Option<i32>,
        #[arg(long)]
        date: Option<String>,
    },
    /// Define one fiscal year: the first year of a company whose fiscal year is not
    /// the calendar year, or the year in which it changes. It runs from `--start`
    /// (first day of a month) to `--end` (last day of a month), at most 18 months
    /// (kirjanpitolaki 1:4); later years continue as 12-month years. Without any
    /// definition fiscal years are calendar years. Append-only: a definition that
    /// would move a sealed voucher to another fiscal year is refused
    /// (`FISCAL_YEAR_CONFLICT`); repeating an identical definition replays.
    Define {
        #[arg(long)]
        start: String,
        #[arg(long)]
        end: String,
        #[arg(long, default_value = "cli")]
        actor: String,
    },
    /// List the fiscal-year definitions (empty: every fiscal year is a calendar year).
    List,
}

#[derive(Subcommand, Debug)]
enum CaptureReceiptCmd {
    /// Project or record an ordinary receipt from strict stdin JSON
    /// (`StructuredReceiptInput`). First run with `--dry-run`; after one user approval,
    /// re-run with the returned hash as `--approved-preparation-hash`.
    #[command(after_help = input_help::CAPTURE_RECEIPT_RECORD)]
    Record {
        #[arg(long, conflicts_with = "approved_preparation_hash")]
        dry_run: bool,
        #[arg(long)]
        approved_preparation_hash: Option<String>,
    },
    /// Start a non-routine consult run from stdin JSON (`CaptureReceiptInput`); parks on
    /// the `ApproveConsultDisclosure` preview gate.
    #[command(after_help = input_help::CAPTURE_RECEIPT_START)]
    Start,
    /// Answer a gate: `--gate` is `ApproveConsultDisclosure`, `ApproveVoucher` or
    /// `ApproveKnowledgePR`, and `--answer` is `approve` or `reject`.
    Answer {
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        gate: String,
        #[arg(long, default_value = "approve")]
        answer: String,
    },
    /// Resume a run whose consult parked on a quota `Effect::Wait`.
    Resume {
        #[arg(long)]
        run_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum HuntReceiptsCmd {
    /// Start a run from stdin JSON (`HuntReceiptsInput`); drives the targeted pulls.
    #[command(after_help = input_help::HUNT_RECEIPTS_START)]
    Start,
    /// Answer a hunt gate, giving the gate ID the run returned as `--gate`. An
    /// `ApproveMatch:…` gate takes `--answer approve` or `--answer discard`; a
    /// `Clarify:…` gate takes `--answer acknowledge` or `--answer discard`.
    Answer {
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        gate: String,
        #[arg(long)]
        answer: String,
    },
    /// Resume a run whose pull parked on a quota `Effect::Wait`.
    Resume {
        #[arg(long)]
        run_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum ObligationCmd {
    /// List all obligations (open + resolved).
    List,
    /// Resolve an open obligation by id.
    Resolve {
        #[arg(long)]
        id: String,
    },
}

#[derive(Subcommand, Debug)]
enum IntakeCmd {
    /// List the typed match candidates.
    List,
}

#[derive(Subcommand, Debug)]
enum EmployeeCmd {
    /// Add (or replace) an employee from stdin JSON (`EmployeeState`: employee_id, name,
    /// hetu, iban, tax_card). PII lands only in the separate PII store. YTD is derived
    /// from the earnings ledger, not part of the record — re-adding never resets it.
    #[command(after_help = input_help::EMPLOYEE_ADD)]
    Add,
    /// Show one employee's master data (`--employee <id>`). Pass `--tax-year <YYYY>`
    /// to include that year's derived year-to-date gross (`ytd_gross`).
    Show {
        #[arg(long)]
        employee: String,
        #[arg(long)]
        tax_year: Option<i32>,
    },
    /// List known employee ids (non-PII).
    List,
}

#[derive(Subcommand, Debug)]
enum RunPayrollCmd {
    /// Start a run from stdin JSON (`PayrollIntent`: payment_date + per-employee
    /// gross/tulolaji); computes every payslip and parks on the `ApproveRun` gate.
    #[command(after_help = input_help::RUN_PAYROLL_START)]
    Start,
    /// Approve (or reject) the batch: `--answer approve` or `--answer reject`.
    /// Approve drives every employee's payment + report.
    ApproveRun {
        #[arg(long)]
        run_id: String,
        #[arg(long, default_value = "approve")]
        answer: String,
    },
    /// Re-run a parked compute after fixing an employee's master data (ResolveErrors).
    ResolveErrors {
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        employee: String,
    },
    /// Deliberately re-drive a parked employee chain after a payment/report failure
    /// (HandlePaymentFailure). Never automatic.
    HandlePaymentFailure {
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        employee: String,
    },
    /// Resume an interrupted run after a crash: reconcile every employee chain to the
    /// durable state and re-drive the incomplete ones (idempotent).
    Resume {
        #[arg(long)]
        run_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum FileVatCmd {
    /// Derive numbered filing fields and separate payment instructions from a
    /// period's sealed vouchers, and reconcile its VAT accounts with those figures.
    /// Read-only: creates no Run and reads no Vero credentials or network; works on
    /// filed periods too. On an open or closing period it is a draft for checking
    /// before the close (`fileable: false`, attention VAT_PERIOD_NOT_SEALED).
    Preview {
        #[arg(long)]
        period: PeriodId,
    },
    /// Record evidence after the operator has filed manually. Does not file or pay.
    CompleteManual {
        #[arg(long)]
        period: PeriodId,
        #[arg(long, value_enum)]
        channel: CliManualVatChannel,
        #[arg(long)]
        evidence_ref: String,
        /// Required when a Vero API attempt for this period is OutcomeUnknown.
        #[arg(long)]
        remote_status_evidence_ref: Option<String>,
        /// The operator replaced an already filed return (OmaVero): record it as
        /// a further filing of the period. Without this flag a filed period is
        /// refused; with it, a period that is not filed is (VAT_PERIOD_NOT_FILED).
        #[arg(long)]
        replacement: bool,
    },
    /// Start a `file-vat` Run from stdin JSON (period + sales_vat/purchase_vat +
    /// payee_iban); reconciles and parks on the `ApproveFiling` preview gate.
    /// Untagged VAT-account postings refuse with VAT_ACCOUNTS_UNRECONCILED unless
    /// `vat_reconciliation_acknowledgement` {reason, vouchers} names exactly them.
    #[command(after_help = input_help::FILE_VAT_START)]
    Start,
    /// Start a `file-vat` Run that replaces the period's filed return, from the
    /// `file-vat start` stdin JSON plus `replacement_reasons`. The whole return is
    /// sent again from the sealed vouchers, including later vouchers sealed with
    /// `corrects_vat_period` for this period. Parks on `ApproveFiling` with the
    /// amounts the replacement changes; approve, reconcile and status work as for
    /// a first return. The Run pays nothing.
    #[command(after_help = input_help::FILE_VAT_REPLACE)]
    Replace,
    /// Approve (or reject) the filing preview gate: `--answer approve` or
    /// `--answer reject`.
    /// The submit-filing effect's `outcome` carries the Vero receipt
    /// (`unique_identifier`, `accepted_timestamp`) once accepted, or names
    /// `file-vat reconcile` when the outcome is unknown; `run show` reads it later.
    #[command(after_long_help = VERO_TRANSPORT_HELP)]
    ApproveFiling {
        #[arg(long)]
        run_id: String,
        #[arg(long, default_value = "approve")]
        answer: String,
    },
    /// Approve (or reject) the payment preview gate. Approving pays through a
    /// bank payment adapter; a route without one (the direct Vero.fi route) is
    /// refused with VAT_PAYMENT_ROUTE_UNAVAILABLE before anything is recorded.
    ApprovePayment {
        #[arg(long)]
        run_id: String,
        #[arg(long, default_value = "approve")]
        answer: String,
    },
    /// Record that the user pays a filed return's tax themselves, outside
    /// aggountant, and complete the Run: answers an open ApprovePayment gate
    /// `user-pays` and skips the payment effect. For Runs left waiting at
    /// ApprovePayment on the direct Vero.fi route (newer Runs there finish on
    /// their own). Sends nothing and does not claim the tax is paid; `run show`
    /// keeps the amount, account and due date. Refused before the filing is
    /// accepted or once a bank payment was sent; repeating it changes nothing.
    UserPays {
        #[arg(long)]
        run_id: String,
    },
    /// Reconcile a filing parked on OUTCOME_UNKNOWN through GetFiledVATReturn/v2
    /// (read only). A matching processed return is adopted; every answer is
    /// recorded on the Run, and two "not found" answers at least one Finnish bank
    /// day apart close it as not filed so a new Run may file the period.
    #[command(after_long_help = VERO_TRANSPORT_HELP)]
    Reconcile {
        #[arg(long)]
        run_id: String,
    },
    /// Read-only: what happened to the period's VAT return. A receipted direct
    /// filing is compared with GetFiledVATReturn/v2 (received, pending authority
    /// processing, processed); other cases are answered from local records. Never
    /// submits or writes; a failed read is VAT_STATUS_UNKNOWN, never "not filed".
    #[command(after_long_help = VERO_TRANSPORT_HELP)]
    Status {
        #[arg(long)]
        period: PeriodId,
    },
    /// Deliberately retry only the separate payment effect. A Vero filing is never
    /// blindly re-submitted.
    Retry {
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        effect: String,
    },
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum CliManualVatChannel {
    Omavero,
    Ilmoitin,
}

impl CliManualVatChannel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Omavero => "omavero",
            Self::Ilmoitin => "ilmoitin",
        }
    }
}

#[derive(Subcommand, Debug)]
enum PeriodCmd {
    /// Show a period's authoritative state (`Open`/`Closing`/`Sealed`).
    Show {
        #[arg(long)]
        period: String,
    },
}

#[derive(Subcommand, Debug)]
enum RunCmd {
    /// Start a Run: `run start close-period --period 2026-03`. Moves the period
    /// to Closing at once and returns the ApproveClose gate with `open_gate.content`
    /// (vouchers, income statement, closing balances, open obligations), the
    /// `message` and the `next_command` that answers it.
    Start {
        /// Workflow name (only `close-period`).
        workflow: String,
        #[arg(long)]
        period: String,
    },
    /// Preview a close without changing anything: `run preview close-period
    /// --period 2026-03` returns the content ApproveClose would show, its
    /// `content_hash`, and whether a close can start now (`ready`, `blockers`).
    Preview {
        /// Workflow name (only `close-period`).
        workflow: String,
        #[arg(long)]
        period: String,
    },
    /// Derived view of a Run (folded from its effect states — no stored status).
    /// Offline and read-only. An authority filing effect (`file-vat`
    /// submit-filing, `file-wage` submit-wage, employer-report) carries `outcome`:
    /// `accepted` with the recorded `receipt` (Vero `unique_identifier` and
    /// `accepted_timestamp`; Incomes Register `ir_delivery_id`, `ir_report_id`,
    /// `version`), `outcome_unknown` with the `next_command` that settles it, or
    /// `rejected` with its `error_code`. A rejected effect carries its
    /// `error_code`. Status `not_filed` (`file-vat`) is final: the Run stopped
    /// before its filing was attempted and sent nothing.
    Show {
        #[arg(long)]
        run_id: String,
    },
    /// Answer a gate: `run answer --run-id ... --gate ApproveClose --answer approve`.
    Answer {
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        gate: String,
        #[arg(long)]
        answer: String,
    },
    /// Resume a Run (reconciles process state to authoritative fiscal state). A
    /// `file-vat` Run that stopped while reconciling, before any filing step
    /// began, is settled `not_filed` (VAT_INTERRUPTED_BEFORE_FILING on its
    /// reconcile step); one whose filing may have begun is refused with
    /// VAT_FILING_UNRESOLVED (use `file-vat status`). Other `file-vat` Runs are
    /// returned unchanged.
    Resume {
        #[arg(long)]
        run_id: String,
    },
    /// List known Run ids.
    List,
    /// Migrate a Run to this binary's workflow version (`WORKFLOW_VERSION_MISMATCH`).
    Migrate {
        #[arg(long)]
        run_id: String,
        /// Required acknowledgement that the definition changed.
        #[arg(long)]
        confirm: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ChartCmd {
    /// List accounts active at a date (S1: single chart version).
    List {
        #[arg(long)]
        at: Option<String>,
    },
}

const VOUCHER_SHOW_HELP: &str = r#"EXAMPLE
  aggountant --home ./accounting --json voucher show 1

Returns the exact strict-decoded canonical body and its voucher hash. Stored
provenance is reported separately because it is not part of the fiscal hash preimage.
This read does not verify the full chain; use `voucher verify` for that."#;

const VOUCHER_LIST_HELP: &str = r#"EXAMPLES
  aggountant --home ./accounting --json voucher list \
    --attachment-ref aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
  aggountant --home ./accounting --json voucher list \
    --posting-from 2026-01-01 --posting-to 2026-01-31 --limit 25

`--attachment-ref` is exactly 64 lowercase hexadecimal characters. Date bounds must
be supplied together as canonical YYYY-MM-DD. When both filter kinds are present they
use AND semantics. Results are always number ASC and limited to 1..=100 (default 100).
`count` is the number of returned rows; `truncated` reports further matches. A row is
only a possible source match, never an automatic duplicate decision."#;

const VOUCHER_VAT_CODES_HELP: &str = r#"EXAMPLE
  aggountant --json voucher vat-codes --document-date 2026-01-15

Lists every code the seal accepts for that document date: direction, rate class,
percent in force, scheme (normal or margin), whether the draft needs buyer_vat_id,
and whether a -deduct<NN> partial-deduction suffix is accepted. The list comes
from the same parser, guards and dated rate table the seal uses. A chart may still
refuse a generic code on an account that has a dedicated rate class; `voucher
validate` checks that per account. Where the code goes on a voucher: see
`voucher seal --help`."#;

#[derive(Subcommand, Debug)]
enum VoucherCmd {
    /// Build + locally validate a draft from stdin JSON (Σ=0, ≥2 postings).
    #[command(after_help = input_help::VOUCHER_DRAFT)]
    Draft,
    /// Ledger dry-run of a bare draft or the `voucher seal` envelope. No write.
    #[command(after_help = input_help::VOUCHER_VALIDATE)]
    Validate,
    /// Commit: atomic gapless numbering, hash-chain advance, provenance, idempotency.
    #[command(after_help = input_help::VOUCHER_SEAL)]
    Seal,
    /// Show one sealed voucher by its positive voucher number (read-only).
    #[command(after_help = VOUCHER_SHOW_HELP)]
    Show {
        /// Positive sealed voucher number.
        number: u64,
    },
    /// List possible sealed-voucher source matches using bounded exact filters.
    #[command(after_help = VOUCHER_LIST_HELP)]
    List {
        /// Exact attachment sha256: 64 lowercase hexadecimal characters.
        #[arg(long)]
        attachment_ref: Option<String>,
        /// Inclusive posting-date lower bound (canonical YYYY-MM-DD); requires --posting-to.
        #[arg(long)]
        posting_from: Option<String>,
        /// Inclusive posting-date upper bound (canonical YYYY-MM-DD); requires --posting-from.
        #[arg(long)]
        posting_to: Option<String>,
        /// Maximum returned rows, 1..=100.
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Walk and verify the whole chain.
    Verify,
    /// List the vat_code values a posting may carry on a document date, with the
    /// rate in force (offline, read-only; needs no home).
    #[command(after_help = VOUCHER_VAT_CODES_HELP)]
    VatCodes {
        /// The voucher's document_date (YYYY-MM-DD); it selects the dated rate.
        #[arg(long)]
        document_date: String,
    },
}

#[derive(Subcommand, Debug)]
enum ArtifactCmd {
    /// Store a file as a content-addressed blob; prints its sha256 ref.
    Attach {
        #[arg(long)]
        file: PathBuf,
    },
}

/// Structured input for `voucher seal` (stdin JSON).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SealInput {
    draft: DraftInput,
    natural_key: String,
    #[serde(default = "default_actor")]
    actor: String,
}

fn default_actor() -> String {
    "cli".to_string()
}

fn parse_cli_date(raw: &str) -> Result<Date, CliError> {
    Date::parse(raw).map_err(|e| CliError::user(e.error_code(), e.to_string()))
}

/// Structured input for `import camt seal` (stdin JSON): the skill's per-entry
/// account assignments, the entries already booked by vouchers sealed outside
/// the import, plus the sealing actor.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportSealInput {
    assignments: Vec<Assignment>,
    #[serde(default)]
    existing_vouchers: Vec<ExistingVoucher>,
    #[serde(default = "default_actor")]
    actor: String,
}

/// Coarse failure class used by the single error-to-exit mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ErrorClass {
    /// Bad input, not-found, validation, or a resolvable domain-state conflict.
    CallerActionable,
    /// I/O, a missing dependency, or a broken internal invariant.
    System,
}

impl ErrorClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::CallerActionable => "user",
            Self::System => "system",
        }
    }
}

/// The central §2 routing signal. Help and version are successful display events
/// and therefore bypass this failure-only mapping.
fn failure_exit(class: ErrorClass) -> ExitCode {
    match class {
        ErrorClass::CallerActionable => ExitCode::from(1),
        ErrorClass::System => ExitCode::from(2),
    }
}

/// A machine-readable CLI error.
#[derive(Debug)]
struct CliError {
    code: String,
    class: ErrorClass,
    message: String,
    /// Structured context for a caller that branches on more than the code,
    /// emitted as the error's `details` member.
    details: Option<Value>,
}

impl CliError {
    fn user(code: &str, message: impl Into<String>) -> Self {
        CliError {
            code: code.to_string(),
            class: ErrorClass::CallerActionable,
            message: message.into(),
            details: None,
        }
    }
    fn system(code: &str, message: impl Into<String>) -> Self {
        CliError {
            code: code.to_string(),
            class: ErrorClass::System,
            message: message.into(),
            details: None,
        }
    }
    fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}

fn classify_ledger_error(error: &aggountant_ledger::LedgerError) -> ErrorClass {
    use aggountant_ledger::LedgerError::*;

    match error {
        // Persisted chain corruption, unavailable audit, and store failures are
        // broken invariants/infrastructure: changing argv cannot repair them.
        HashChainBroken(_) | VoucherReadCorrupt(_) | AuditUnavailable(_) | Store(_) => {
            ErrorClass::System
        }
        // These name invalid input, not-found, already-exists, validation, or a
        // resolvable domain-state conflict. Keep this exhaustive so a new ledger
        // variant cannot silently acquire the wrong §2 class.
        AccountNotFound(_)
        | VoucherNumberTaken(_)
        | VoucherNotFound(_)
        | InvalidVoucherRead(_)
        | CasConflict(_)
        | PeriodClosing(_)
        | PeriodSealed(_)
        | VoucherNotInRun(_)
        | PeriodTransitionConflict(_)
        | PeriodNotSealed(_)
        | VatPeriodNotFiled(_)
        | ArtifactNotFound(_)
        | IdempotencyKeyReuse(_)
        | AlreadyInitialized(_)
        | MasterDataNotInitialized(_)
        | MasterDataCorrupt(_)
        | ChartVersionChanged(_)
        | EmployeeNotFound(_)
        | FiscalYearConflict(_)
        | Statements(_)
        | InvalidHash(_)
        | Wire(_)
        | Model(_) => ErrorClass::CallerActionable,
    }
}

impl From<aggountant_ledger::LedgerError> for CliError {
    fn from(e: aggountant_ledger::LedgerError) -> Self {
        CliError {
            code: e.error_code().to_string(),
            class: classify_ledger_error(&e),
            message: e.to_string(),
            details: None,
        }
    }
}

impl From<aggountant_domain::DomainError> for CliError {
    fn from(e: aggountant_domain::DomainError) -> Self {
        CliError::user(e.error_code(), e.to_string())
    }
}

impl From<ImportError> for CliError {
    fn from(e: ImportError) -> Self {
        CliError::user(e.error_code(), e.to_string())
    }
}

impl From<aggountant_process::ProcessError> for CliError {
    fn from(e: aggountant_process::ProcessError) -> Self {
        use aggountant_process::ProcessError::*;

        let class = match &e {
            JournalCorrupt(_)
            | KnowledgeHistoryCorrupt(_)
            | Store(_)
            | VatStatusUnknown(_)
            | IncomesRegister {
                code:
                    "EMPLOYER_REPORT_STATUS_UNKNOWN"
                    | "WAGE_CORRECTION_STATUS_UNKNOWN"
                    | "YTJ_UNAVAILABLE"
                    | "YTJ_RESPONSE_INVALID",
                ..
            } => ErrorClass::System,
            Ledger(error) => classify_ledger_error(error),
            RunNotFound(_)
            | WorkflowVersionMismatch(_)
            | RunLeased(_)
            | GateNotOpen(_)
            | InvalidAnswer(_)
            | VatFiguresMismatch(_)
            | VatFilingUnresolved(_)
            | VatPaymentRouteUnavailable(_)
            | VatAccountsUnreconciled(_)
            | VatReconciliationAckMismatch(_)
            | CaptureVatMismatch(_)
            | CaptureFactRequired(_)
            | CaptureClarificationRequired(_)
            | CaptureUnsupportedTreatment(_)
            | CaptureVatClassUnknown(_)
            | CapturePreparationChanged(_)
            | InvalidKnowledge(_)
            | KnowledgeNotFound(_)
            | OpenObligation(_)
            | CloseContentChanged(_)
            | CloseYearBlocked(_)
            | IncomesRegister { .. } => ErrorClass::CallerActionable,
        };
        CliError {
            code: e.error_code().to_string(),
            class,
            message: e.to_string(),
            details: None,
        }
    }
}

/// Parse a `--period` argument into a `PeriodId`, surfacing `INVALID_PERIOD_ID`.
fn parse_period(s: &str) -> Result<PeriodId, CliError> {
    PeriodId::parse(s).map_err(|e| CliError::user(e.error_code(), e.to_string()))
}

fn read_stdin_json<T: for<'de> Deserialize<'de>>() -> Result<T, CliError> {
    parse_stdin_json(&read_stdin_text()?)
}

fn parse_stdin_json<T: for<'de> Deserialize<'de>>(input: &str) -> Result<T, CliError> {
    serde_json::from_str(input)
        .map_err(|e| CliError::user("INVALID_INPUT", format!("could not parse stdin JSON: {e}")))
}

fn read_stdin_text() -> Result<String, CliError> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|e| CliError::system("IO_ERROR", e.to_string()))?;
    let input = String::from_utf8(bytes).map_err(|e| {
        CliError::user(
            "INVALID_INPUT",
            format!("stdin JSON must be valid UTF-8: {e}"),
        )
    })?;
    if input.trim().is_empty() {
        return Err(CliError::user(
            "INVALID_INPUT",
            "expected JSON on stdin, got nothing",
        ));
    }
    Ok(input)
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    home: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vero_credentials_file: Option<PathBuf>,
}

struct ResolvedHome {
    value: PathBuf,
    source: &'static str,
    marker_path: Option<PathBuf>,
}

fn environment_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn absolute_environment_path(name: &str) -> Result<PathBuf, CliError> {
    environment_path(name)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| {
            CliError::system(
                "CONFIG_PATH_UNRESOLVED",
                format!("{name} must be set to an absolute, non-empty path"),
            )
        })
}

fn config_path() -> Result<PathBuf, CliError> {
    #[cfg(target_os = "macos")]
    {
        return Ok(absolute_environment_path("HOME")?
            .join("Library")
            .join("Application Support")
            .join("aggountant")
            .join("config.json"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(path) = environment_path("XDG_CONFIG_HOME").filter(|path| path.is_absolute()) {
            return Ok(path.join("aggountant").join("config.json"));
        }
        return Ok(absolute_environment_path("HOME")?
            .join(".config")
            .join("aggountant")
            .join("config.json"));
    }
    #[cfg(windows)]
    {
        let base = environment_path("APPDATA")
            .filter(|path| path.is_absolute())
            .or_else(|| environment_path("LOCALAPPDATA").filter(|path| path.is_absolute()))
            .or_else(|| {
                environment_path("USERPROFILE")
                    .filter(|path| path.is_absolute())
                    .map(|path| path.join("AppData").join("Roaming"))
            })
            .or_else(|| environment_path("HOME").filter(|path| path.is_absolute()))
            .ok_or_else(|| {
                CliError::system(
                    "CONFIG_PATH_UNRESOLVED",
                    "%APPDATA%, %USERPROFILE%, or $HOME must be an absolute, non-empty path",
                )
            })?;
        return Ok(base.join("aggountant").join("config.json"));
    }
    #[allow(unreachable_code)]
    Err(CliError::system(
        "CONFIG_PATH_UNRESOLVED",
        "unsupported platform for configuration path resolution",
    ))
}

fn non_blank_path(path: &Path, source: &str) -> Result<(), CliError> {
    if path.as_os_str().to_string_lossy().trim().is_empty() {
        return Err(CliError::user(
            "INVALID_HOME",
            format!(
                "{source} must be a non-empty path, got {:?}",
                path.display()
            ),
        ));
    }
    Ok(())
}

fn read_config(path: &Path) -> Result<ConfigFile, CliError> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ConfigFile::default());
        }
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
            return Err(CliError::user(
                "INVALID_CONFIG",
                format!(
                    "configuration file {} is not valid UTF-8: {error}",
                    path.display()
                ),
            ));
        }
        Err(error) => {
            return Err(CliError::system(
                "CONFIG_READ_ERROR",
                format!(
                    "could not read configuration file {}: {error}",
                    path.display()
                ),
            ));
        }
    };
    serde_json::from_str(&content).map_err(|error| {
        CliError::user(
            "INVALID_CONFIG",
            format!(
                "could not parse configuration file {}: {error}",
                path.display()
            ),
        )
    })
}

fn write_config(path: &Path, config: &ConfigFile) -> Result<(), CliError> {
    let parent = path.parent().ok_or_else(|| {
        CliError::system(
            "CONFIG_PATH_UNRESOLVED",
            format!("configuration path {} has no parent", path.display()),
        )
    })?;
    std::fs::create_dir_all(parent).map_err(|_| {
        CliError::system(
            "CONFIG_WRITE_ERROR",
            "platform configuration directory could not be created",
        )
    })?;
    let temporary = path.with_extension("json.tmp");
    let mut bytes = serde_json::to_vec_pretty(config).map_err(|_| {
        CliError::system("CONFIG_WRITE_ERROR", "configuration serialization failed")
    })?;
    bytes.push(b'\n');
    let mut file = std::fs::File::create(&temporary).map_err(|_| {
        CliError::system(
            "CONFIG_WRITE_ERROR",
            "temporary platform configuration could not be created",
        )
    })?;
    file.write_all(&bytes).map_err(|_| {
        CliError::system(
            "CONFIG_WRITE_ERROR",
            "temporary platform configuration could not be written",
        )
    })?;
    file.sync_all().map_err(|_| {
        CliError::system(
            "CONFIG_WRITE_ERROR",
            "temporary platform configuration could not be synced",
        )
    })?;
    std::fs::rename(&temporary, path).map_err(|_| {
        CliError::system(
            "CONFIG_WRITE_ERROR",
            "platform configuration could not be replaced",
        )
    })
}

#[cfg(unix)]
fn parent_is_on_same_filesystem(directory: &Path) -> Result<bool, CliError> {
    use std::os::unix::fs::MetadataExt;

    let Some(parent) = directory.parent() else {
        return Ok(false);
    };
    let current = std::fs::metadata(directory).map_err(|error| {
        CliError::system(
            "CWD_UNRESOLVED",
            format!(
                "could not inspect {} during home discovery: {error}",
                directory.display()
            ),
        )
    })?;
    let parent = std::fs::metadata(parent).map_err(|error| {
        CliError::system(
            "CWD_UNRESOLVED",
            format!(
                "could not inspect {} during home discovery: {error}",
                parent.display()
            ),
        )
    })?;
    Ok(current.dev() == parent.dev())
}

#[cfg(not(unix))]
fn parent_is_on_same_filesystem(directory: &Path) -> Result<bool, CliError> {
    Ok(directory.parent().is_some())
}

fn discover_home() -> Result<Option<(PathBuf, PathBuf)>, CliError> {
    let cwd = std::env::current_dir().map_err(|error| {
        CliError::system(
            "CWD_UNRESOLVED",
            format!("could not resolve current directory for home discovery: {error}"),
        )
    })?;
    for directory in cwd.ancestors() {
        let marker = directory.join(".aggountant");
        if marker.is_dir() {
            return Ok(Some((directory.to_path_buf(), marker)));
        }
        // An ancestor repository is an explicit boundary: a marker above it
        // belongs to a different project, not this invocation's data root.
        if directory.join(".git").exists() || !parent_is_on_same_filesystem(directory)? {
            break;
        }
    }
    Ok(None)
}

/// Per-key precedence for the data home: --home > $AGGOUNTANT_HOME > config
/// `home` > nearest .aggountant marker > ./accounting.
fn resolve_explicit_home(cli: &Cli) -> Result<Option<ResolvedHome>, CliError> {
    if let Some(home) = &cli.home {
        non_blank_path(home, "--home")?;
        return Ok(Some(ResolvedHome {
            value: home.clone(),
            source: "flag",
            marker_path: None,
        }));
    }
    if let Some(home) = std::env::var_os("AGGOUNTANT_HOME").map(PathBuf::from) {
        non_blank_path(&home, "$AGGOUNTANT_HOME")?;
        return Ok(Some(ResolvedHome {
            value: home,
            source: "env",
            marker_path: None,
        }));
    }
    Ok(None)
}

fn resolve_home(cli: &Cli) -> Result<ResolvedHome, CliError> {
    if let Some(home) = resolve_explicit_home(cli)? {
        return Ok(home);
    }
    match config_path() {
        Ok(path) => resolve_home_from_config(&path),
        Err(_) => resolve_discovered_or_default(),
    }
}

fn resolve_home_from_config(path: &Path) -> Result<ResolvedHome, CliError> {
    if let Some(mut home) = read_config(path)?.home {
        non_blank_path(
            &home,
            format!("configuration key 'home' in {}", path.display()).as_str(),
        )?;
        if home.is_relative() {
            let parent = path.parent().ok_or_else(|| {
                CliError::system(
                    "CONFIG_PATH_UNRESOLVED",
                    format!("configuration path {} has no parent", path.display()),
                )
            })?;
            home = parent.join(home);
        }
        return Ok(ResolvedHome {
            value: home,
            source: "file",
            marker_path: None,
        });
    }
    resolve_discovered_or_default()
}

fn resolve_discovered_or_default() -> Result<ResolvedHome, CliError> {
    if let Some((home, marker_path)) = discover_home()? {
        return Ok(ResolvedHome {
            value: home,
            source: "discovered",
            marker_path: Some(marker_path),
        });
    }
    Ok(ResolvedHome {
        value: PathBuf::from("accounting"),
        source: "default",
        marker_path: None,
    })
}

/// The resolved home for `init`, the one command that may create it. Every
/// other command takes its home from [`existing_home`].
fn home_path(cli: &Cli) -> Result<PathBuf, CliError> {
    Ok(resolve_home(cli)?.value)
}

/// The resolved home, provided `init` set it up.
///
/// `Ledger::open` and the process stores create whatever they miss (the
/// directory, `fiscal.sqlite`, `artifacts/`, the PII store, `process/`), so a
/// wrong `--home`, a command run from the wrong directory or the `./accounting`
/// fallback would otherwise leave a new empty home behind and answer from it as
/// if the books were empty. Only `init` creates a home; everything else stops
/// here, writes nothing, and says which path it looked at and why that one.
///
/// A home is the voucher database plus the reference masterdata `init` wrote.
/// Without `fiscal.sqlite` (for example a fresh clone of the company
/// repository, which ignores it) opening would start empty books. `artifacts/`
/// and the PII store are not required: an empty `artifacts/` does not survive
/// a git checkout or some copies, and recreating either inside a real home is
/// what opening has always done.
fn existing_home(cli: &Cli) -> Result<PathBuf, CliError> {
    let resolved = resolve_home(cli)?;
    let home = &resolved.value;
    let initialized =
        home.join("fiscal.sqlite").is_file() && home.join("masterdata").join("reference").is_dir();
    if initialized {
        return Ok(resolved.value);
    }
    // A relative path is shown as the directory it names, so a wrong working
    // directory is visible in the message itself.
    let shown = std::path::absolute(home).unwrap_or_else(|_| home.clone());
    let chosen_by = match (resolved.source, &resolved.marker_path) {
        ("flag", _) => "the --home option".to_string(),
        ("env", _) => "$AGGOUNTANT_HOME".to_string(),
        ("file", _) => match config_path() {
            Ok(path) => format!("the `home` key of {}", path.display()),
            Err(_) => "the `home` key of the configuration file".to_string(),
        },
        ("discovered", Some(marker)) => format!("the .aggountant marker {}", marker.display()),
        _ => "the ./accounting fallback, used when no --home, $AGGOUNTANT_HOME, configured home or .aggountant marker applies".to_string(),
    };
    Err(CliError::user(
        "HOME_NOT_INITIALIZED",
        format!(
            "no initialized aggountant home at {}, chosen by {chosen_by}; nothing was written. \
             Point the command at the company home (`aggountant config show` shows how the home is chosen), \
             or run `aggountant --home {} init` to set up a new company there",
            shown.display(),
            shown.display(),
        ),
    )
    .with_details(json!({
        "home": shown.display().to_string(),
        "source": resolved.source,
        "marker_path": resolved.marker_path.map(|path| path.display().to_string()),
    })))
}

/// Open the ledger of the resolved, already initialized home.
fn open_existing_ledger(cli: &Cli) -> Result<Ledger, CliError> {
    Ledger::open(&existing_home(cli)?).map_err(Into::into)
}

fn parse_voucher_list_query(
    attachment_ref: &Option<String>,
    posting_from: &Option<String>,
    posting_to: &Option<String>,
    limit: usize,
) -> Result<VoucherListQuery, CliError> {
    let attachment_ref = attachment_ref
        .as_deref()
        .map(Hash::parse_hex)
        .transpose()
        .map_err(|error| CliError::user(error.error_code(), error.to_string()))?;
    let posting_from = posting_from
        .as_deref()
        .map(Date::parse)
        .transpose()
        .map_err(|error| CliError::user(error.error_code(), error.to_string()))?;
    let posting_to = posting_to
        .as_deref()
        .map(Date::parse)
        .transpose()
        .map_err(|error| CliError::user(error.error_code(), error.to_string()))?;
    if limit == 0 || limit > 100 {
        return Err(CliError::user(
            "INVALID_VOUCHER_READ",
            format!("limit must be in 1..=100, got {limit}"),
        ));
    }
    match (posting_from, posting_to) {
        (Some(from), Some(to)) if from > to => {
            return Err(CliError::user(
                "INVALID_VOUCHER_READ",
                format!("posting date range is reversed: {from} is after {to}"),
            ));
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(CliError::user(
                "INVALID_VOUCHER_READ",
                "posting date filtering requires both --posting-from and --posting-to",
            ));
        }
        (None, None) if attachment_ref.is_none() => {
            return Err(CliError::user(
                "INVALID_VOUCHER_READ",
                "voucher list requires --attachment-ref or both posting date bounds",
            ));
        }
        _ => {}
    }
    Ok(VoucherListQuery {
        attachment_ref,
        posting_from,
        posting_to,
        limit,
    })
}

fn draft_to_json(draft: &VoucherDraft) -> Value {
    serde_json::to_value(draft).expect("VoucherDraft serializes")
}

fn run(cli: &Cli) -> Result<Value, CliError> {
    match &cli.command {
        Command::FileWage { cmd } => wage::execute(cmd, &existing_home(cli)?),
        Command::Claim { cmd } => claim::run(cmd),
        Command::Tulorekisteri { cmd } => tulorekisteri::execute(cmd, &existing_home(cli)?),
        Command::Config { cmd } => match cmd {
            ConfigCmd::Path => Ok(json!({
                "schema_version": 1,
                "config_path": config_path()?.display().to_string(),
            })),
            ConfigCmd::Show => {
                let config_path = config_path()?;
                let platform_config = read_config(&config_path)?;
                let home = match resolve_explicit_home(cli)? {
                    Some(home) => home,
                    None => resolve_home_from_config(&config_path)?,
                };
                let credential_path = resolve_credential_path(&platform_config, None)?;
                Ok(json!({
                    "schema_version": 1,
                    "config_path": config_path.display().to_string(),
                    "config": {
                        "home": {
                            "value": home.value.display().to_string(),
                            "source": home.source,
                            "marker_path": home.marker_path.map(|path| path.display().to_string()),
                        },
                        "vero_credentials_file": {
                            "value": credential_path.value.map(|path| path.display().to_string()),
                            "source": credential_path.source,
                            "secret": false,
                        },
                    },
                }))
            }
        },
        Command::VeroFi { cmd } => {
            let home = existing_home(cli)?;
            let platform_path = config_path()?;
            let mut platform_config = read_config(&platform_path)?;
            match cmd {
                VeroFiCmd::Status => {
                    let credential_path = resolve_credential_path(&platform_config, None)?;
                    let report = evaluate_vero_readiness(
                        &home,
                        read_vero_profile(&home),
                        credential_path.value.as_deref(),
                        VeroReadinessPurpose::ProductionFiling,
                        aggountant_model::Instant::now(),
                    );
                    let mut value = serde_json::to_value(report).map_err(|_| {
                        CliError::system("INTERNAL_ERROR", "readiness output failed")
                    })?;
                    value["schema_version"] = json!(1);
                    Ok(value)
                }
                VeroFiCmd::Configure { dry_run, replace } => {
                    let input = read_stdin_text()?;
                    let profile_file = aggountant_broker::vero_profile_path(&home);
                    let stored = match std::fs::read(&profile_file) {
                        Ok(bytes) => Some(bytes),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                        Err(_) => {
                            return Err(CliError::system(
                                "VERO_PROFILE_READ_ERROR",
                                format!("{} could not be read", profile_file.display()),
                            ))
                        }
                    };
                    let outcome =
                        vero_fi::apply_configure_input(stored.as_deref(), &input, *replace)?;
                    let profile = outcome.profile;
                    let credential_path = resolve_credential_path(
                        &platform_config,
                        outcome.credential_path.as_deref(),
                    )?;
                    let report = evaluate_vero_readiness(
                        &home,
                        Ok(profile.clone()),
                        credential_path.value.as_deref(),
                        VeroReadinessPurpose::ProductionFiling,
                        aggountant_model::Instant::now(),
                    );
                    if profile.mode == VeroFilingMode::Production {
                        report.require_ready().map_err(|blockers| {
                            CliError::user(
                                "VERO_NOT_READY",
                                format!("production configuration refused: {blockers}"),
                            )
                        })?;
                    }
                    let stored_credential_path = platform_config.vero_credentials_file.clone();
                    if !dry_run {
                        if outcome.credential_path.is_some() {
                            platform_config.vero_credentials_file = outcome.credential_path.clone();
                            write_config(&platform_path, &platform_config)?;
                        }
                        write_vero_profile(&home, &profile).map_err(|message| {
                            CliError::system("VERO_PROFILE_WRITE_ERROR", message)
                        })?;
                    }
                    let mut value = serde_json::to_value(report).map_err(|_| {
                        CliError::system("INTERNAL_ERROR", "readiness output failed")
                    })?;
                    value["schema_version"] = json!(1);
                    value["dry_run"] = json!(dry_run);
                    value["written"] = json!(!dry_run);
                    value["merged"] = json!(!replace);
                    let mut changes = outcome.changes;
                    if let Some(path) = outcome
                        .credential_path
                        .as_ref()
                        .filter(|path| stored_credential_path.as_ref() != Some(*path))
                    {
                        changes.push(json!({
                            "field": "credential_path",
                            "before": stored_credential_path,
                            "after": path,
                        }));
                    }
                    value["changes"] = json!(changes);
                    if *dry_run {
                        let mut would = vec![json!({
                            "action": "update",
                            "resource": "vero_filing_profile",
                            "path": profile_file,
                        })];
                        if outcome.credential_path.is_some() {
                            would.push(json!({
                                "action": "update",
                                "resource": "platform_config",
                                "path": platform_path,
                                "field": "vero_credentials_file",
                            }));
                        }
                        value["would"] = json!(would);
                    }
                    Ok(value)
                }
                VeroFiCmd::Verify {
                    target: VeroVerifyTarget::Test,
                } => {
                    let credential_path = resolve_credential_path(&platform_config, None)?;
                    let mut profile = read_vero_profile(&home)
                        .map_err(|message| CliError::user("INVALID_VERO_PROFILE", message))?;
                    let report = evaluate_vero_readiness(
                        &home,
                        Ok(profile.clone()),
                        credential_path.value.as_deref(),
                        VeroReadinessPurpose::TestVerification,
                        aggountant_model::Instant::now(),
                    );
                    report
                        .require_ready()
                        .map_err(|blockers| CliError::user("VERO_TEST_NOT_READY", blockers))?;
                    let credential_path = credential_path
                        .value
                        .expect("ready requires credential path");
                    perform_test_verification(
                        &home,
                        &credential_path,
                        &mut profile,
                        report,
                        VeroCredentialTarget::Test.base_url(),
                        aggountant_model::Instant::now(),
                    )
                }
                VeroFiCmd::VerifyPortal => {
                    let credential_path = resolve_credential_path(&platform_config, None)?;
                    let mut profile = read_vero_profile(&home)
                        .map_err(|message| CliError::user("INVALID_VERO_PROFILE", message))?;
                    let report = evaluate_vero_readiness(
                        &home,
                        Ok(profile.clone()),
                        credential_path.value.as_deref(),
                        VeroReadinessPurpose::PortalVerification,
                        aggountant_model::Instant::now(),
                    );
                    report.require_ready().map_err(|blockers| {
                        CliError::user("VERO_PORTAL_VERIFICATION_NOT_READY", blockers)
                    })?;
                    perform_portal_verification(
                        &home,
                        &credential_path
                            .value
                            .expect("ready requires credential path"),
                        &mut profile,
                        report,
                        aggountant_model::Instant::now(),
                    )
                }
            }
        }
        Command::Explain { code, lang } => explain::execute(code.as_deref(), *lang),
        Command::Skill { cmd } => match cmd {
            SkillCmd::List => Ok(skill::list()),
            SkillCmd::Install { name, target } => {
                skill::ensure_known(name.as_deref())?;
                let targets = match target {
                    Some(target) => vec![target.clone()],
                    None => skill::default_targets()?,
                };
                skill::install(name.as_deref(), &targets)
            }
            SkillCmd::Print { name, file } => skill::printable(name, file.as_deref()),
        },
        Command::Init => {
            let home = home_path(cli)?;
            // `Ledger::init` is the single refuse-to-clobber gate (it returns
            // ALREADY_INITIALIZED); no pre-check, so there is no TOCTOU window.
            let ledger = Ledger::init(&home)?;
            let accounts = ledger.chart_list(None)?;
            Ok(json!({
                "home": home.display().to_string(),
                "masterdata_path": home.join("masterdata").join("reference").display().to_string(),
                "chart_version": ledger.chart_version()?,
                "account_count": accounts.len(),
            }))
        }
        Command::Chart { cmd } => match cmd {
            ChartCmd::List { at } => {
                let at = match at {
                    Some(s) => Some(
                        Date::parse(s)
                            .map_err(|e| CliError::user(e.error_code(), e.to_string()))?,
                    ),
                    None => None,
                };
                let ledger = open_existing_ledger(cli)?;
                let accounts = ledger.chart_list(at)?;
                Ok(json!({
                    "chart_version": ledger.chart_version()?,
                    "accounts": serde_json::to_value(&accounts).expect("accounts serialize"),
                }))
            }
        },
        Command::Voucher { cmd } => match cmd {
            VoucherCmd::Draft => {
                let input: DraftInput = read_stdin_json()?;
                let draft = build_draft(input)?;
                Ok(draft_to_json(&draft))
            }
            VoucherCmd::Validate => {
                let input = read_stdin_text()?;
                // A seal envelope has a top-level `draft`, `natural_key` or `actor`;
                // a bare draft never does (its fields are closed). The envelope is
                // read exactly as `seal` reads it, so "validate this file, then seal
                // this file" holds and a broken envelope gets seal's error.
                let is_envelope =
                    serde_json::from_str::<serde_json::Value>(&input).is_ok_and(|v| {
                        ["draft", "natural_key", "actor"]
                            .iter()
                            .any(|field| v.get(field).is_some())
                    });
                let report = if is_envelope {
                    let input: SealInput = parse_stdin_json(&input)?;
                    let draft = build_draft(input.draft)?;
                    let ledger = open_existing_ledger(cli)?;
                    ledger.validate_seal(&draft, &input.natural_key)?
                } else {
                    let draft: VoucherDraft = parse_stdin_json(&input)?;
                    let ledger = open_existing_ledger(cli)?;
                    ledger.validate(&draft)?
                };
                Ok(serde_json::to_value(report).expect("report serializes"))
            }
            VoucherCmd::Seal => {
                let input: SealInput = read_stdin_json()?;
                let draft = build_draft(input.draft)?;
                let ledger = open_existing_ledger(cli)?;
                let sealed = ledger.seal(
                    &draft,
                    &SealRequest {
                        natural_key: input.natural_key,
                        actor: input.actor,
                        policy_set: PolicySet::default(),
                    },
                )?;
                Ok(serde_json::to_value(sealed).expect("sealed voucher serializes"))
            }
            VoucherCmd::Show { number } => {
                if *number == 0 {
                    return Err(CliError::user(
                        "INVALID_VOUCHER_READ",
                        "voucher number must be a positive integer, got 0",
                    ));
                }
                let ledger = open_existing_ledger(cli)?;
                let mut value = serde_json::to_value(ledger.voucher(*number)?)
                    .expect("voucher view serializes");
                value["schema_version"] = json!(1);
                Ok(value)
            }
            VoucherCmd::List {
                attachment_ref,
                posting_from,
                posting_to,
                limit,
            } => {
                let query =
                    parse_voucher_list_query(attachment_ref, posting_from, posting_to, *limit)?;
                let ledger = open_existing_ledger(cli)?;
                let result = ledger.list_vouchers(&query)?;
                Ok(json!({
                    "schema_version": 1,
                    "filters": {
                        "attachment_ref": attachment_ref,
                        "posting_from": posting_from,
                        "posting_to": posting_to,
                    },
                    "limit": limit,
                    "order": result.order,
                    "count": result.count,
                    "truncated": result.truncated,
                    "vouchers": result.vouchers,
                }))
            }
            VoucherCmd::VatCodes { document_date } => {
                let date = parse_cli_date(document_date)?;
                Ok(json!({
                    "schema_version": 1,
                    "document_date": date,
                    "codes": aggountant_domain::vat_codes_on(date),
                }))
            }
            VoucherCmd::Verify => {
                let ledger = open_existing_ledger(cli)?;
                let report = ledger.verify()?;
                Ok(serde_json::to_value(report).expect("verify report serializes"))
            }
        },
        Command::Artifact { cmd } => match cmd {
            ArtifactCmd::Attach { file } => {
                let bytes = read_input_file_bytes(file)?;
                let ledger = open_existing_ledger(cli)?;
                let addr = ledger.attach_artifact(&bytes)?;
                Ok(json!({ "artifact_ref": addr.to_hex(), "size": bytes.len() }))
            }
        },
        Command::Period { cmd } => match cmd {
            PeriodCmd::Show { period } => {
                let period = parse_period(period)?;
                let ledger = open_existing_ledger(cli)?;
                let view = ledger.period_view(&period)?;
                Ok(serde_json::to_value(view).expect("period view serializes"))
            }
        },
        Command::Run { cmd } => {
            let engine = ProcessEngine::open(&existing_home(cli)?);
            match cmd {
                RunCmd::Start { workflow, period } => {
                    require_close_period(workflow)?;
                    let period = parse_period(period)?;
                    let view = engine.start_close_period(&period, &PolicySet::default())?;
                    close_period_json(&engine, view)
                }
                RunCmd::Preview { workflow, period } => {
                    require_close_period(workflow)?;
                    let period = parse_period(period)?;
                    Ok(serde_json::to_value(engine.preview_close_period(&period)?)
                        .expect("close-period preview serializes"))
                }
                RunCmd::Show { run_id } => close_period_json(&engine, engine.show(run_id)?),
                RunCmd::Answer {
                    run_id,
                    gate,
                    answer,
                } => close_period_json(&engine, engine.answer(run_id, gate, answer)?),
                RunCmd::Resume { run_id } => close_period_json(&engine, engine.resume(run_id)?),
                RunCmd::List => Ok(json!({ "runs": engine.list()? })),
                RunCmd::Migrate { run_id, confirm } => {
                    if !confirm {
                        return Err(CliError::user(
                            "CONFIRM_REQUIRED",
                            "run migrate rewrites the Run's workflow version — pass --confirm",
                        ));
                    }
                    Ok(serde_json::to_value(engine.migrate(run_id)?).expect("run view serializes"))
                }
            }
        }
        Command::FileVat { cmd } => {
            let home = existing_home(cli)?;
            let engine = ProcessEngine::open(&home);
            // Select the deterministic scripted seam or the readiness-gated live path
            // once for this command. Live operations retain the exact profile which
            // passed readiness instead of reading vero-filing.json again downstream.
            let broker_script = std::env::var_os("AGGOUNTANT_BROKER_SCRIPT");
            match cmd {
                FileVatCmd::Preview { period } => Ok(serde_json::to_value(
                    engine.preview_offline_vat(period, &PolicySet::default())?,
                )
                .expect("offline VAT preview serializes")),
                FileVatCmd::CompleteManual {
                    period,
                    channel,
                    evidence_ref,
                    remote_status_evidence_ref,
                    replacement,
                } => Ok(serde_json::to_value(if *replacement {
                    engine.complete_manual_vat_replacement(
                        period,
                        channel.as_str(),
                        evidence_ref,
                        remote_status_evidence_ref.as_deref(),
                    )?
                } else {
                    engine.complete_manual_vat(
                        period,
                        channel.as_str(),
                        evidence_ref,
                        remote_status_evidence_ref.as_deref(),
                    )?
                })
                .expect("manual VAT completion serializes")),
                FileVatCmd::Start => {
                    let input: CliFileVatInput = read_stdin_json()?;
                    let operation = build_file_vat_broker(&home, broker_script.as_deref())?;
                    let input = resolve_file_vat_input(input, operation.profile.as_ref())?;
                    let started =
                        engine.start_file_vat(&input, &operation.broker, &PolicySet::default())?;
                    Ok(serde_json::to_value(started).expect("file-vat start serializes"))
                }
                FileVatCmd::Replace => {
                    let input: CliFileVatInput = read_stdin_json()?;
                    let operation = build_file_vat_broker(&home, broker_script.as_deref())?;
                    let input = resolve_file_vat_input(input, operation.profile.as_ref())?;
                    let started = engine.start_file_vat_replacement(
                        &input,
                        &operation.broker,
                        &PolicySet::default(),
                    )?;
                    Ok(serde_json::to_value(started).expect("file-vat replace serializes"))
                }
                FileVatCmd::ApproveFiling { run_id, answer } => {
                    let operation = build_file_vat_broker(&home, broker_script.as_deref())?;
                    if let Some(profile) = &operation.profile {
                        engine.ensure_file_vat_profile_matches(run_id, profile)?;
                    }
                    let view = engine.answer_file_vat(
                        run_id,
                        "ApproveFiling",
                        answer,
                        &operation.broker,
                    )?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
                FileVatCmd::ApprovePayment { run_id, answer } => {
                    let operation = build_file_vat_broker(&home, broker_script.as_deref())?;
                    let view = engine.answer_file_vat(
                        run_id,
                        "ApprovePayment",
                        answer,
                        &operation.broker,
                    )?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
                FileVatCmd::UserPays { run_id } => {
                    // Records a local fact only: no credentials or readiness are
                    // needed, and the broker is used solely to read the outbox for
                    // an earlier bank payment attempt.
                    let broker = Broker::new(&home, Vault::new(), Registry::new());
                    let view = engine.leave_file_vat_payment_to_user(run_id, &broker)?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
                FileVatCmd::Reconcile { run_id } => {
                    let operation = build_file_vat_broker(&home, broker_script.as_deref())?;
                    let view = engine.reconcile_file_vat(run_id, &operation.broker)?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
                FileVatCmd::Status { period } => {
                    let status = match engine.file_vat_status_plan(period)? {
                        FileVatStatusPlan::Answered(status) => status,
                        FileVatStatusPlan::Query(query) => {
                            // Only a receipted or interrupted filing needs credentials. A
                            // broker that cannot be built leaves the status unknown, not
                            // "not filed".
                            let operation = build_file_vat_broker(&home, broker_script.as_deref())
                                .map_err(|e| CliError {
                                    code: "VAT_STATUS_UNKNOWN".into(),
                                    class: e.class,
                                    message: format!(
                                        "cannot read the Tax Administration's view of {period} \
                                         ({}: {}); {}, do not file again",
                                        e.code,
                                        e.message,
                                        query.unread_meaning()
                                    ),
                                    details: None,
                                })?;
                            engine.query_file_vat_status(&query, &operation.broker)?
                        }
                    };
                    Ok(serde_json::to_value(status).expect("file-vat status serializes"))
                }
                FileVatCmd::Retry { run_id, effect } => {
                    let operation = build_file_vat_broker(&home, broker_script.as_deref())?;
                    let view = engine.retry_file_vat_submit(run_id, effect, &operation.broker)?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
            }
        }
        Command::Employee { cmd } => {
            let ledger = open_existing_ledger(cli)?;
            match cmd {
                EmployeeCmd::Add => {
                    let state: EmployeeState = read_stdin_json()?;
                    ledger.upsert_employee(&state)?;
                    Ok(json!({ "employee_id": state.employee_id, "stored": true }))
                }
                EmployeeCmd::Show { employee, tax_year } => {
                    let state = ledger.get_employee(employee)?;
                    let mut value = serde_json::to_value(state).expect("employee serializes");
                    if let Some(year) = tax_year {
                        let ytd = ledger.ytd_gross(employee, *year)?;
                        value["tax_year"] = json!(year);
                        value["ytd_gross"] = serde_json::to_value(&ytd).expect("money serializes");
                    }
                    Ok(value)
                }
                EmployeeCmd::List => Ok(json!({ "employees": ledger.list_employees()? })),
            }
        }
        Command::RunPayroll { cmd } => {
            let home = existing_home(cli)?;
            let engine = ProcessEngine::open(&home);
            match cmd {
                RunPayrollCmd::Start => {
                    let intent: PayrollIntent = read_stdin_json()?;
                    let view = engine.start_run_payroll(&intent, &PolicySet::default())?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
                RunPayrollCmd::ApproveRun { run_id, answer } => {
                    let broker = build_broker(&home)?;
                    let view = engine.answer_run_payroll(run_id, answer, &broker)?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
                RunPayrollCmd::ResolveErrors { run_id, employee } => {
                    let view = engine.resolve_errors(run_id, employee)?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
                RunPayrollCmd::HandlePaymentFailure { run_id, employee } => {
                    let broker = build_broker(&home)?;
                    let view = engine.handle_payment_failure(run_id, employee, &broker)?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
                RunPayrollCmd::Resume { run_id } => {
                    let broker = build_broker(&home)?;
                    let view = engine.resume_run_payroll(run_id, &broker)?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
            }
        }
        Command::CaptureReceipt { cmd } => {
            let home = existing_home(cli)?;
            match cmd {
                CaptureReceiptCmd::Record {
                    dry_run,
                    approved_preparation_hash,
                } => {
                    let input: StructuredReceiptInput = read_stdin_json()?;
                    let ledger = Ledger::open(&home)?;
                    if *dry_run {
                        let mut value =
                            serde_json::to_value(prepare_structured_receipt(&ledger, &input)?)
                                .expect("prepared receipt serializes");
                        value["dry_run"] = json!(true);
                        Ok(value)
                    } else {
                        let approved = approved_preparation_hash.as_deref().ok_or_else(|| {
                            CliError::user(
                                "APPROVAL_REQUIRED",
                                "recording requires --approved-preparation-hash from a successful --dry-run",
                            )
                        })?;
                        let sealed = record_structured_receipt(&ledger, &input, approved)?;
                        let mut value =
                            serde_json::to_value(sealed).expect("sealed voucher serializes");
                        value["dry_run"] = json!(false);
                        value["approved_preparation_hash"] = json!(approved);
                        Ok(value)
                    }
                }
                CaptureReceiptCmd::Start => {
                    let input: CaptureReceiptInput = read_stdin_json()?;
                    let engine = ProcessEngine::open(&home);
                    let view = engine.start_capture_receipt(&input, &PolicySet::default())?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
                CaptureReceiptCmd::Answer {
                    run_id,
                    gate,
                    answer,
                } => {
                    let engine = ProcessEngine::open(&home);
                    let broker = build_broker(&home)?;
                    let view = engine.answer_capture_receipt(
                        run_id,
                        gate,
                        answer,
                        &broker,
                        &SystemClock,
                    )?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
                CaptureReceiptCmd::Resume { run_id } => {
                    let engine = ProcessEngine::open(&home);
                    let broker = build_broker(&home)?;
                    let view = engine.resume_capture_receipt(run_id, &broker, &SystemClock)?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
            }
        }
        Command::HuntReceipts { cmd } => {
            let home = existing_home(cli)?;
            let engine = ProcessEngine::open(&home);
            match cmd {
                HuntReceiptsCmd::Start => {
                    let input: HuntReceiptsInput = read_stdin_json()?;
                    let broker = build_broker(&home)?;
                    let view = engine.start_hunt_receipts(
                        &input,
                        &broker,
                        &PolicySet::default(),
                        &SystemClock,
                    )?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
                HuntReceiptsCmd::Answer {
                    run_id,
                    gate,
                    answer,
                } => {
                    let view = engine.answer_hunt(run_id, gate, answer)?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
                HuntReceiptsCmd::Resume { run_id } => {
                    let broker = build_broker(&home)?;
                    let view = engine.resume_hunt_receipts(run_id, &broker, &SystemClock)?;
                    Ok(serde_json::to_value(view).expect("run view serializes"))
                }
            }
        }
        Command::Obligation { cmd } => {
            let engine = ProcessEngine::open(&existing_home(cli)?);
            match cmd {
                ObligationCmd::List => {
                    let obligations = engine.obligations().list()?;
                    Ok(json!({
                        "obligations": serde_json::to_value(&obligations).expect("serialize"),
                    }))
                }
                ObligationCmd::Resolve { id } => {
                    let o = engine.obligations().resolve(id)?;
                    Ok(serde_json::to_value(o).expect("obligation serializes"))
                }
            }
        }
        Command::Knowledge { cmd } => knowledge::run(cmd, &existing_home(cli)?),
        Command::Home { cmd } => home::run(cmd, &existing_home(cli)?),
        Command::Intake { cmd } => {
            let home = existing_home(cli)?;
            match cmd {
                IntakeCmd::List => {
                    let intake = aggountant_process::Intake::open(&home);
                    Ok(json!({
                        "candidates": serde_json::to_value(intake.list()?).expect("serialize"),
                    }))
                }
            }
        }
        Command::CloseYear { cmd } => {
            let engine = ProcessEngine::open(&existing_home(cli)?);
            if let CloseYearCmd::Start { dry_run: true } = cmd {
                let input: CloseYearInput = read_stdin_json()?;
                let preview = engine.preview_close_year(&input, &PolicySet::default())?;
                return Ok(serde_json::to_value(preview).expect("close-year preview serializes"));
            }
            let (value, run_id, reported_by) = match cmd {
                CloseYearCmd::Start { .. } => {
                    let input: CloseYearInput = read_stdin_json()?;
                    let started = engine.start_close_year(&input, &PolicySet::default())?;
                    let run_id = started.run.run_id.clone();
                    (
                        serde_json::to_value(started).expect("close-year start serializes"),
                        run_id,
                        CloseYearNextFor::Other,
                    )
                }
                CloseYearCmd::Answer {
                    run_id,
                    gate,
                    answer,
                } => (
                    serde_json::to_value(engine.answer_close_year(run_id, gate, answer)?)
                        .expect("run view serializes"),
                    run_id.clone(),
                    CloseYearNextFor::Other,
                ),
                CloseYearCmd::SealCorrection { run_id, correction } => (
                    serde_json::to_value(engine.seal_close_year_correction(run_id, correction)?)
                        .expect("run view serializes"),
                    run_id.clone(),
                    CloseYearNextFor::Other,
                ),
                CloseYearCmd::Resume { run_id } => (
                    serde_json::to_value(engine.resume_close_year(run_id)?)
                        .expect("run view serializes"),
                    run_id.clone(),
                    CloseYearNextFor::Other,
                ),
                CloseYearCmd::Show { run_id } => (
                    serde_json::to_value(engine.show_close_year(run_id)?)
                        .expect("close-year show serializes"),
                    run_id.clone(),
                    CloseYearNextFor::Show,
                ),
            };
            // Every close-year command says what to do next and what approving the
            // open gate seals, so the agent need not infer it from the gate list.
            let next = engine.close_year_next(&run_id, reported_by)?;
            let mut value = value;
            let object = value
                .as_object_mut()
                .expect("close-year output is an object");
            object.insert("message".into(), Value::String(next.message));
            object.insert(
                "next_command".into(),
                next.next_command.map_or(Value::Null, Value::String),
            );
            Ok(value)
        }
        Command::Report { cmd } => reports::run(cmd, &existing_home(cli)?),
        Command::FiscalYear { cmd } => {
            let ledger = open_existing_ledger(cli)?;
            match cmd {
                FiscalYearCmd::Show { year, date } => {
                    let year = match (year, date) {
                        (Some(year), _) => *year,
                        (None, Some(date)) => ledger.fiscal_year_of(parse_cli_date(date)?)?.year(),
                        (None, None) => unreachable!("clap requires --year or --date"),
                    };
                    let view = ledger.fiscal_year_view(year)?;
                    Ok(serde_json::to_value(view).expect("fiscal-year view serializes"))
                }
                FiscalYearCmd::Define { start, end, actor } => {
                    let view = ledger.define_fiscal_year(
                        parse_cli_date(start)?,
                        parse_cli_date(end)?,
                        actor,
                    )?;
                    Ok(serde_json::to_value(view).expect("fiscal-year definition serializes"))
                }
                FiscalYearCmd::List => {
                    let calendar = ledger.fiscal_calendar()?;
                    let definitions: Vec<Value> = calendar
                        .defined()
                        .iter()
                        .map(|year| {
                            json!({
                                "year": year.year(),
                                "start": year.start(),
                                "end": year.end(),
                                "months": year.months(),
                            })
                        })
                        .collect();
                    Ok(json!({
                        "calendar_years": definitions.is_empty(),
                        "definitions": definitions,
                    }))
                }
            }
        }
        Command::Import { cmd } => match cmd {
            ImportCmd::Camt { cmd } => match cmd {
                CamtCmd::Parse { file } => {
                    let xml = read_camt_file(file)?;
                    let statement = parse_camt(&xml)?;
                    Ok(serde_json::to_value(statement).expect("parsed statement serializes"))
                }
                CamtCmd::Seal {
                    file,
                    dry_run,
                    approved_preparation_hash,
                } => bank_import::seal_camt(
                    cli,
                    file,
                    *dry_run,
                    approved_preparation_hash.as_deref(),
                ),
                CamtCmd::Observations { attachment_ref } => {
                    bank_import::observations(cli, attachment_ref.as_deref())
                }
            },
        },
    }
}

/// Read a camt.053 or camt.052 source file to a UTF-8 string for the `parse` (read-only) path.
fn read_camt_file(path: &std::path::Path) -> Result<String, CliError> {
    let raw = read_file_bytes(path)?;
    decode_camt(&raw, path)
}

/// Read a caller-selected input file. A missing path is actionable by changing
/// the invocation; failures reading an existing path remain system I/O errors.
fn read_input_file_bytes(path: &Path) -> Result<Vec<u8>, CliError> {
    std::fs::read(path).map_err(|error| {
        let message = format!("could not read {}: {error}", path.display());
        if error.kind() == std::io::ErrorKind::NotFound {
            CliError::user("FILE_NOT_FOUND", message)
        } else {
            CliError::system("IO_ERROR", message)
        }
    })
}

fn read_file_bytes(path: &Path) -> Result<Vec<u8>, CliError> {
    read_input_file_bytes(path)
}

/// Decode camt bytes as UTF-8. A non-UTF-8 file is malformed user input
/// (a wrong/unsupported encoding), not a system failure — `INVALID_CAMT`.
fn decode_camt(raw: &[u8], path: &std::path::Path) -> Result<String, CliError> {
    // Strip a UTF-8 BOM if present so the XML declaration parses; the retained
    // attachment still uses the original bytes.
    let bytes = raw.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(raw);
    String::from_utf8(bytes.to_vec()).map_err(|e| {
        CliError::user(
            "INVALID_CAMT",
            format!("{} is not valid UTF-8 XML: {e}", path.display()),
        )
    })
}

fn resolve_file_vat_input(
    input: CliFileVatInput,
    ready_profile: Option<&VeroFilingProfile>,
) -> Result<FileVatInput, CliError> {
    let (business_id, contact_name, contact_phone) = if let Some(profile) = ready_profile {
        // `build_file_vat_broker` has already passed this exact profile through
        // ProductionFiling readiness, whose invariant requires all identity fields.
        let business_id = profile
            .business_id
            .clone()
            .expect("ready profile requires BusinessId");
        let contact_name = profile
            .contact_name
            .clone()
            .expect("ready profile requires contact name");
        let contact_phone = profile
            .contact_phone
            .clone()
            .expect("ready profile requires contact phone");
        if input
            .business_id
            .as_ref()
            .is_some_and(|value| value != &business_id)
            || input
                .contact_name
                .as_ref()
                .is_some_and(|value| value != &contact_name)
            || input
                .contact_phone
                .as_ref()
                .is_some_and(|value| value != &contact_phone)
        {
            return Err(CliError::user(
                "VERO_FILING_PROFILE_MISMATCH",
                "stdin filing identity conflicts with vero-filing.json",
            ));
        }
        (business_id, contact_name, contact_phone)
    } else {
        (
            input.business_id.ok_or_else(|| {
                CliError::user(
                    "INVALID_INPUT",
                    "business_id is required for the scripted seam",
                )
            })?,
            input.contact_name.ok_or_else(|| {
                CliError::user(
                    "INVALID_INPUT",
                    "contact_name is required for the scripted seam",
                )
            })?,
            input.contact_phone.ok_or_else(|| {
                CliError::user(
                    "INVALID_INPUT",
                    "contact_phone is required for the scripted seam",
                )
            })?,
        )
    };
    Ok(FileVatInput {
        period: input.period,
        sales_vat: input.sales_vat,
        purchase_vat: input.purchase_vat,
        business_id,
        contact_name,
        contact_phone,
        payee_iban: input.payee_iban,
        vat_reconciliation_acknowledgement: input.vat_reconciliation_acknowledgement,
        replacement_reasons: input.replacement_reasons,
    })
}

/// Build a Broker from `$AGGOUNTANT_BROKER_SCRIPT` (a JSON file of scripted authority
/// responses per service). Without a script, the direct Vero adapter requires the shared
/// local production-readiness evaluation and otherwise refuses with `VERO_NOT_READY`.
/// The script's shape per service is a list of
/// `{"status": N, "body": {..}}`, `{"timeout": true}`, or `{"refused": "msg"}`.
fn build_broker(home: &std::path::Path) -> Result<Broker, CliError> {
    if let Some(path) = std::env::var_os("AGGOUNTANT_BROKER_SCRIPT") {
        return build_scripted_broker_path(home, &path);
    }
    build_live_vero_broker(home, read_vero_profile(home)).map(|(broker, _profile)| broker)
}

/// Broker plus the exact readiness-approved filing profile for one `file-vat`
/// operation. `profile == None` identifies the explicit scripted test seam.
struct FileVatBroker {
    broker: Broker,
    profile: Option<VeroFilingProfile>,
}

fn build_file_vat_broker(
    home: &Path,
    script_path: Option<&OsStr>,
) -> Result<FileVatBroker, CliError> {
    if let Some(path) = script_path {
        return Ok(FileVatBroker {
            broker: build_scripted_broker_path(home, path)?,
            profile: None,
        });
    }

    let (broker, profile) = build_live_vero_broker(home, read_vero_profile(home))?;
    Ok(FileVatBroker {
        broker,
        profile: Some(profile),
    })
}

/// Set once this process has built a broker from `$AGGOUNTANT_BROKER_SCRIPT`. The
/// command's output then says so (`test_seam`), because with the seam a filing can
/// be recorded as made although nothing was sent to the authority.
static TEST_SEAM_USED: AtomicBool = AtomicBool::new(false);

const TEST_SEAM_MESSAGE: &str = "AGGOUNTANT_BROKER_SCRIPT is set: authority responses came from a test script and nothing was sent to the authority; a filing recorded now is not a real filing";

/// The `test_seam` marker added to a command's output, or `None` when the command
/// did not use the scripted broker.
fn test_seam_marker() -> Option<Value> {
    TEST_SEAM_USED.load(Ordering::SeqCst).then(|| {
        json!({
            "active": true,
            "variable": "AGGOUNTANT_BROKER_SCRIPT",
            "message": TEST_SEAM_MESSAGE,
        })
    })
}

fn build_scripted_broker_path(home: &Path, path: &OsStr) -> Result<Broker, CliError> {
    TEST_SEAM_USED.store(true, Ordering::SeqCst);
    let raw = std::fs::read_to_string(path).map_err(|e| {
        CliError::system(
            "IO_ERROR",
            format!("could not read broker script {path:?}: {e}"),
        )
    })?;
    let script: Value = serde_json::from_str(&raw)
        .map_err(|e| CliError::user("INVALID_INPUT", format!("broker script JSON: {e}")))?;
    build_scripted_broker(home, &script)
}

fn build_scripted_broker(home: &Path, script: &Value) -> Result<Broker, CliError> {
    let mut registry = Registry::new();
    let mut vault = Vault::new();
    for service in [
        SVC_VERO_VEROTILI,
        SVC_SYNTHETIC_SUBMIT,
        SVC_VERO_VAT_PERIODS,
        SVC_VERO_FILE_VAT_RETURN,
        SVC_VERO_FILED_VAT_RETURN,
        SVC_PANKKI_MAKSU,
        SVC_TULOREKISTERI,
        SVC_EMAIL_PULL,
        SVC_KAUPPATILI_PULL,
        SVC_CONSULT_LLM,
    ] {
        vault.set(service, "sandbox-secret");
        let transport: Box<dyn Transport> = Box::new(ScriptedTransport::new(parse_script_steps(
            script.get(service),
        )?));
        match service {
            SVC_VERO_VEROTILI => registry.register(Box::new(VeroVerotiliAdapter::new(transport))),
            SVC_SYNTHETIC_SUBMIT => {
                registry.register(Box::new(SyntheticSubmitAdapter::new(transport)))
            }
            SVC_VERO_VAT_PERIODS => {
                registry.register(Box::new(ScriptedVeroVatPeriodsAdapter::new(transport)))
            }
            SVC_VERO_FILE_VAT_RETURN => {
                registry.register(Box::new(ScriptedVeroFileVatReturnAdapter::new(transport)))
            }
            SVC_VERO_FILED_VAT_RETURN => {
                registry.register(Box::new(ScriptedVeroFiledVatReturnAdapter::new(transport)))
            }
            SVC_PANKKI_MAKSU => registry.register(Box::new(PankkiMaksuAdapter::new(transport))),
            SVC_TULOREKISTERI => registry.register(Box::new(TulorekisteriAdapter::new(transport))),
            SVC_EMAIL_PULL => registry.register(Box::new(EmailPullAdapter::new(transport))),
            SVC_KAUPPATILI_PULL => {
                registry.register(Box::new(KauppatiliPullAdapter::new(transport)))
            }
            SVC_CONSULT_LLM => {
                registry.register_stream(Box::new(ConsultLlmAdapter::new(transport)))
            }
            _ => unreachable!(),
        }
    }
    Ok(Broker::new(home, vault, registry))
}

fn build_live_vero_broker(
    home: &Path,
    profile: Result<VeroFilingProfile, String>,
) -> Result<(Broker, VeroFilingProfile), CliError> {
    let platform_config = read_config(&config_path()?)?;
    let credential_path = resolve_credential_path(&platform_config, None)?;
    let report = evaluate_vero_readiness(
        home,
        profile.clone(),
        credential_path.value.as_deref(),
        VeroReadinessPurpose::ProductionFiling,
        aggountant_model::Instant::now(),
    );
    report.require_ready().map_err(|blockers| {
        CliError::user(
            "VERO_NOT_READY",
            format!("production filing blocked: {blockers}"),
        )
    })?;
    let broker = build_direct_vero_broker(
        home,
        &credential_path
            .value
            .expect("ready requires credential path"),
        VeroCredentialTarget::Production,
    )?;
    Ok((
        broker,
        profile.expect("ready requires a valid filing profile"),
    ))
}

fn perform_test_verification(
    home: &Path,
    credential_path: &Path,
    profile: &mut VeroFilingProfile,
    report: aggountant_broker::VeroReadinessReport,
    base_url: &str,
    observed_at: aggountant_model::Instant,
) -> Result<Value, CliError> {
    let broker = build_direct_vero_broker_at(home, credential_path, base_url)?;
    let business_id = profile
        .business_id
        .as_ref()
        .expect("ready requires identity")
        .as_str()
        .to_string();
    let request = serde_json::to_string(&json!({
        "BusinessId": business_id,
        "FilingYear": observed_at.utc().year().to_string(),
    }))
    .map_err(|_| CliError::system("INTERNAL_ERROR", "verification request failed"))?;
    let envelope = aggountant_broker::Envelope::new(
        SVC_VERO_VAT_PERIODS,
        "GetVATPeriods/v1",
        format!("verify-test:{business_id}"),
        json!({"request_bytes": request}),
    );
    broker
        .fetch(&envelope)
        .map_err(|error| CliError::system("VERO_TEST_VERIFICATION_FAILED", error.to_string()))?;
    let fingerprint = report
        .certificate
        .as_ref()
        .expect("ready requires certificate")
        .fingerprint_sha256
        .clone();
    profile.attestations.test_verification = Some(aggountant_broker::TestVerificationEvidence {
        observed_at,
        certificate_fingerprint_sha256: fingerprint.clone(),
    });
    write_vero_profile(home, profile)
        .map_err(|message| CliError::system("VERO_PROFILE_WRITE_ERROR", message))?;
    Ok(json!({
        "schema_version": 1,
        "verified": true,
        "target": "test",
        "operation": "GetVATPeriods/v1",
        "business_id": business_id,
        "observed_at": observed_at,
        "certificate_fingerprint_sha256": fingerprint,
        "mode": profile.mode,
    }))
}

fn perform_portal_verification(
    home: &Path,
    credential_path: &Path,
    profile: &mut VeroFilingProfile,
    report: aggountant_broker::VeroReadinessReport,
    observed_at: aggountant_model::Instant,
) -> Result<Value, CliError> {
    use aggountant_broker::vero_vat_api::{
        portal_file_vat_return_request, portal_get_filed_vat_return_request,
        portal_get_vat_periods_request, PORTAL_VERIFICATION_OPERATIONS,
    };

    let broker = build_portal_verification_broker(home, credential_path)?;
    let envelope = |service: &str, operation: &str, body: Vec<u8>| {
        aggountant_broker::Envelope::new(
            service,
            operation,
            format!(
                "verify-portal:{}:{operation}",
                observed_at.inner().as_microsecond()
            ),
            json!({"request_bytes": String::from_utf8(body).expect("static JSON is UTF-8")}),
        )
    };
    broker
        .fetch(&envelope(
            SVC_VERO_VAT_PERIODS,
            PORTAL_VERIFICATION_OPERATIONS[0],
            portal_get_vat_periods_request(),
        ))
        .map_err(|error| CliError::system("VERO_PORTAL_VERIFICATION_FAILED", error.to_string()))?;
    let filing = envelope(
        SVC_VERO_FILE_VAT_RETURN,
        PORTAL_VERIFICATION_OPERATIONS[1],
        portal_file_vat_return_request(),
    );
    let receipt = match broker
        .submit(
            &format!("verify-portal:{}", observed_at.inner().as_microsecond()),
            filing.natural_key.as_str(),
            &filing,
        )
        .map_err(|error| CliError::system("VERO_PORTAL_VERIFICATION_FAILED", error.to_string()))?
    {
        RemoteOutcome::Delivered { receipt } => receipt,
        RemoteOutcome::Rejected { code, message } => {
            return Err(CliError::user(&code, message));
        }
        RemoteOutcome::Unknown => {
            return Err(CliError::system(
                "VERO_PORTAL_VERIFICATION_UNKNOWN",
                "FileVATReturn verification did not return a definitive receipt",
            ));
        }
        RemoteOutcome::RecoveredAlreadyProcessed { .. } => {
            return Err(CliError::system(
                "VERO_PORTAL_VERIFICATION_FAILED",
                "verification endpoint returned an unsupported recovery outcome",
            ));
        }
    };
    broker
        .fetch(&envelope(
            SVC_VERO_FILED_VAT_RETURN,
            PORTAL_VERIFICATION_OPERATIONS[2],
            portal_get_filed_vat_return_request(),
        ))
        .map_err(|error| CliError::system("VERO_PORTAL_VERIFICATION_FAILED", error.to_string()))?;

    let fingerprint = report
        .certificate
        .as_ref()
        .expect("ready requires certificate")
        .fingerprint_sha256
        .clone();
    profile.attestations.production_verification_completed = true;
    profile.attestations.portal_verification =
        Some(aggountant_broker::vero_filing::PortalVerificationEvidence {
            observed_at,
            certificate_fingerprint_sha256: fingerprint.clone(),
            operations: PORTAL_VERIFICATION_OPERATIONS
                .iter()
                .map(|operation| (*operation).to_string())
                .collect(),
            file_vat_return_receipt: receipt.clone(),
        });
    write_vero_profile(home, profile)
        .map_err(|message| CliError::system("VERO_PROFILE_WRITE_ERROR", message))?;
    Ok(json!({
        "schema_version": 1,
        "verified": true,
        "kind": "api_portal_integration_verification",
        "operations": PORTAL_VERIFICATION_OPERATIONS,
        "observed_at": observed_at,
        "certificate_fingerprint_sha256": fingerprint,
        "file_vat_return_receipt": receipt,
        "mode": profile.mode,
        "tax_filing": false,
    }))
}

fn build_portal_verification_broker(
    home: &Path,
    credential_path: &Path,
) -> Result<Broker, CliError> {
    let secret = std::fs::read_to_string(credential_path).map_err(|_| {
        CliError::system(
            "CREDENTIAL_UNAVAILABLE",
            "Vero credential file could not be read",
        )
    })?;
    let timeout = std::time::Duration::from_secs(30);
    let mut registry = Registry::new();
    registry.register(Box::new(
        VeroVatPeriodsAdapter::new_portal_verification(timeout)
            .map_err(|e| CliError::system("BROKER_NOT_CONFIGURED", e.to_string()))?,
    ));
    registry.register(Box::new(
        VeroFileVatReturnAdapter::new_portal_verification(timeout)
            .map_err(|e| CliError::system("BROKER_NOT_CONFIGURED", e.to_string()))?,
    ));
    registry.register(Box::new(
        VeroFiledVatReturnAdapter::new_portal_verification(timeout)
            .map_err(|e| CliError::system("BROKER_NOT_CONFIGURED", e.to_string()))?,
    ));
    let mut vault = Vault::new();
    for service in [
        SVC_VERO_VAT_PERIODS,
        SVC_VERO_FILE_VAT_RETURN,
        SVC_VERO_FILED_VAT_RETURN,
    ] {
        vault.set(service, secret.clone());
    }
    Ok(Broker::new(home, vault, registry))
}

fn build_direct_vero_broker(
    home: &Path,
    credential_path: &Path,
    target: VeroCredentialTarget,
) -> Result<Broker, CliError> {
    build_direct_vero_broker_at(home, credential_path, target.base_url())
}

fn build_direct_vero_broker_at(
    home: &Path,
    credential_path: &Path,
    base_url: &str,
) -> Result<Broker, CliError> {
    let secret = std::fs::read_to_string(credential_path).map_err(|_| {
        CliError::system(
            "CREDENTIAL_UNAVAILABLE",
            "Vero credential file could not be read",
        )
    })?;
    let timeout = std::time::Duration::from_secs(30);
    let mut registry = Registry::new();
    registry.register(Box::new(
        VeroVatPeriodsAdapter::new(base_url, timeout)
            .map_err(|e| CliError::system("BROKER_NOT_CONFIGURED", e.to_string()))?,
    ));
    registry.register(Box::new(
        VeroFileVatReturnAdapter::new(base_url, timeout)
            .map_err(|e| CliError::system("BROKER_NOT_CONFIGURED", e.to_string()))?,
    ));
    registry.register(Box::new(
        VeroFiledVatReturnAdapter::new(base_url, timeout)
            .map_err(|e| CliError::system("BROKER_NOT_CONFIGURED", e.to_string()))?,
    ));
    let mut vault = Vault::new();
    for service in [
        SVC_VERO_VAT_PERIODS,
        SVC_VERO_FILE_VAT_RETURN,
        SVC_VERO_FILED_VAT_RETURN,
    ] {
        vault.set(service, secret.clone());
    }
    Ok(Broker::new(home, vault, registry))
}

/// Parse one service's scripted-response list into transport outcomes.
fn parse_script_steps(
    entry: Option<&Value>,
) -> Result<Vec<Result<RawResponse, TransportError>>, CliError> {
    let Some(list) = entry.and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for step in list {
        if step.get("timeout").and_then(|v| v.as_bool()) == Some(true) {
            out.push(Err(TransportError::Timeout));
        } else if let Some(msg) = step.get("refused").and_then(|v| v.as_str()) {
            out.push(Err(TransportError::ConnectionRefused(msg.to_string())));
        } else if let Some(status) = step.get("status").and_then(|v| v.as_u64()) {
            let body = step.get("body").cloned().unwrap_or(Value::Null);
            out.push(Ok(RawResponse::new(status as u16, body)));
        } else {
            return Err(CliError::user(
                "INVALID_INPUT",
                "broker script step must be {status,body} | {timeout} | {refused}",
            ));
        }
    }
    Ok(out)
}

fn require_close_period(workflow: &str) -> Result<(), CliError> {
    if workflow != "close-period" {
        return Err(CliError::user(
            "UNKNOWN_WORKFLOW",
            format!("`run start` and `run preview` take only `close-period`, got {workflow:?}"),
        ));
    }
    Ok(())
}

/// A Run view as JSON; a `close-period` Run waiting at `ApproveClose` also
/// carries the gate's content, `message` and `next_command`.
fn close_period_json(engine: &ProcessEngine, view: RunView) -> Result<Value, CliError> {
    Ok(serde_json::to_value(engine.close_period_view(view)?).expect("run view serializes"))
}

/// The `message` and `next_command` lines of a close-year output.
fn close_year_next_human(value: &Value) -> String {
    format!(
        "{}\nnext: {}",
        value["message"].as_str().unwrap_or(""),
        value["next_command"].as_str().unwrap_or("-")
    )
}

/// The human rendering of close-period content (`run preview`, or the open
/// `ApproveClose` gate).
fn close_period_content_human(content: &Value) -> String {
    let mut out = format!(
        "period {} ({}..{}): {} voucher(s) #{}..#{}",
        content["period"].as_str().unwrap_or("?"),
        content["first_date"].as_str().unwrap_or("?"),
        content["last_date"].as_str().unwrap_or("?"),
        content["vouchers"]["count"],
        content["vouchers"]["first_number"],
        content["vouchers"]["last_number"],
    );
    let income = &content["income_statement"];
    out.push_str(
        "
income statement:",
    );
    for row in income["rows"].as_array().into_iter().flatten() {
        out.push_str(&format!(
            "
  {}\t{}",
            row["label"].as_str().unwrap_or("?"),
            row["amount"].as_str().unwrap_or("?")
        ));
    }
    out.push_str(&format!(
        "
  result {}, since fiscal year start {}",
        income["result"].as_str().unwrap_or("?"),
        income["year_to_date_result"].as_str().unwrap_or("?")
    ));
    let balance = &content["balance_sheet"];
    out.push_str(&format!(
        "
balances at {}:",
        balance["date"].as_str().unwrap_or("?")
    ));
    for account in balance["accounts"].as_array().into_iter().flatten() {
        out.push_str(&format!(
            "
  {} {}\t{}",
            account["account"].as_str().unwrap_or("?"),
            account["account_name"].as_str().unwrap_or("?"),
            account["balance"].as_str().unwrap_or("?")
        ));
    }
    out.push_str(&format!(
        "
  assets {}, equity and liabilities {} (balanced {})",
        balance["assets_total"].as_str().unwrap_or("?"),
        balance["equity_liabilities_total"].as_str().unwrap_or("?"),
        balance["balanced"]
    ));
    if let Some(note) = balance["opening_note"].as_str() {
        out.push_str(&format!("\n  {note}"));
    }
    for obligation in content["open_obligations"].as_array().into_iter().flatten() {
        out.push_str(&format!(
            "
open obligation {}: {}",
            obligation["id"].as_str().unwrap_or("?"),
            obligation["description"].as_str().unwrap_or("?")
        ));
    }
    out.push_str(&format!(
        "
{}",
        content["locks"].as_str().unwrap_or("")
    ));
    out
}

/// A short human line for non-`--json` mode (best-effort; JSON is the contract).
fn human_line(command: &Command, value: &Value) -> String {
    match command {
        Command::Claim { cmd } => claim::human_line(cmd, value),
        Command::Config { cmd } => match cmd {
            ConfigCmd::Path => value["config_path"].as_str().unwrap_or("?").to_string(),
            ConfigCmd::Show => format!(
                "home {} ({})",
                value["config"]["home"]["value"].as_str().unwrap_or("?"),
                value["config"]["home"]["source"].as_str().unwrap_or("?")
            ),
        },
        Command::Tulorekisteri { .. } => {
            let blockers = value["blockers"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let mut out = format!(
                "Tulorekisteri {} — {} blocker(s)",
                if value["ready"].as_bool() == Some(true) {
                    "ready"
                } else {
                    "not ready"
                },
                blockers.len()
            );
            for b in blockers {
                out.push_str(&format!(
                    "\n{} ({}): {}",
                    b["code"].as_str().unwrap_or("?"),
                    b["owner"].as_str().unwrap_or("?"),
                    b["message"].as_str().unwrap_or("?")
                ));
            }
            out
        }
        Command::VeroFi { cmd } => match cmd {
            VeroFiCmd::Status | VeroFiCmd::Configure { .. } => format!(
                "Vero.fi {} — {} blocker(s)",
                if value["ready"].as_bool() == Some(true) {
                    "ready"
                } else {
                    "not ready"
                },
                value["blockers"].as_array().map_or(0, Vec::len)
            ),
            VeroFiCmd::Verify { .. } => "Vero.fi test verification passed".into(),
            VeroFiCmd::VerifyPortal => "Vero.fi API Portal verification passed".into(),
        },
        Command::Explain { .. } => explain::human(value),
        Command::Skill { cmd } => match cmd {
            SkillCmd::List => value["skills"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|skill| {
                    format!(
                        "{} — {}",
                        skill["name"].as_str().unwrap_or("?"),
                        skill["description"].as_str().unwrap_or("?")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            SkillCmd::Install { .. } => format!(
                "installed {} skill file(s) into {}",
                value["count"],
                value["targets"]
                    .as_array()
                    .map(|targets| {
                        targets
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_else(|| "?".to_string())
            ),
            // Text `skill print` bypasses this line formatter in `main` so its
            // bytes stay identical to the bundled and installed file.
            SkillCmd::Print { .. } => value["content"].as_str().unwrap_or("").to_string(),
        },
        Command::Init => format!(
            "initialized {} — chart_version {}, {} accounts",
            value["home"].as_str().unwrap_or("?"),
            value["chart_version"].as_str().unwrap_or("?"),
            value["account_count"]
        ),
        Command::Chart { .. } => {
            let accounts = value["accounts"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let mut out = format!(
                "chart_version {} — {} accounts",
                value["chart_version"].as_str().unwrap_or("?"),
                accounts.len()
            );
            for account in accounts {
                out.push_str(&format!(
                    "\n{}\t{}\t{}",
                    account["id"].as_str().unwrap_or("?"),
                    account["name"].as_str().unwrap_or("?"),
                    account["kind"].as_str().unwrap_or("?")
                ));
            }
            out
        }
        Command::Voucher { cmd } => match cmd {
            VoucherCmd::Seal => format!(
                "sealed voucher #{} hash {} (replayed={})",
                value["number"], value["hash"], value["replayed"]
            ),
            VoucherCmd::Show { .. } => format!(
                "voucher #{} hash {}",
                value["number"], value["voucher_hash"]
            ),
            VoucherCmd::List { .. } => format!(
                "{} returned possible source match(es), number ASC (truncated={})",
                value["count"], value["truncated"]
            ),
            VoucherCmd::VatCodes { .. } => {
                let mut out = format!(
                    "vat codes on {}",
                    value["document_date"].as_str().unwrap_or("?")
                );
                for code in value["codes"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
                    out.push_str(&format!(
                        "\n{}\t{} %",
                        code["code"].as_str().unwrap_or("?"),
                        code["percent"].as_str().unwrap_or("?")
                    ));
                }
                out
            }
            VoucherCmd::Verify => format!(
                "chain ok — {} vouchers, head #{}",
                value["voucher_count"], value["head_number"]
            ),
            _ => value.to_string(),
        },
        Command::Artifact { .. } => format!("artifact_ref {}", value["artifact_ref"]),
        Command::Period { .. } => format!(
            "period {} — {} (owner {}, vat_filed {})",
            value["period"], value["state"], value["owning_run_id"], value["vat_filed"]
        ),
        Command::Run { cmd } => match cmd {
            RunCmd::List => format!("runs: {}", value["runs"]),
            RunCmd::Preview { .. } => format!(
                "close preview {} — {} (ready {})\n{}\n{}\nnext: {}",
                value["period"],
                value["state"],
                value["ready"],
                close_period_content_human(&value["content"]),
                value["message"].as_str().unwrap_or(""),
                value["next_command"].as_str().unwrap_or("-")
            ),
            _ => {
                let mut line = format!(
                    "run {} — {} [{}]",
                    value["run_id"], value["status"], value["period"]
                );
                if let Some(gate) = value.get("open_gate") {
                    line.push_str(&format!(
                        "\ngate {} (bound {})\n{}\n{}\nnext: {}",
                        gate["gate_id"].as_str().unwrap_or("?"),
                        gate["bound"],
                        close_period_content_human(&gate["content"]),
                        value["message"].as_str().unwrap_or(""),
                        value["next_command"].as_str().unwrap_or("-")
                    ));
                }
                line
            }
        },
        Command::FileVat { cmd } => match cmd {
            FileVatCmd::Preview { .. } => {
                let mut line = format!(
                    "VAT preview {}{} — net {}, pay separately {}, VAT accounts {}",
                    value["period"],
                    if value["fileable"] == true {
                        ""
                    } else {
                        " (draft, not fileable)"
                    },
                    value["net_vat"],
                    value["payment_amount"],
                    value["vat_reconciliation"]["status"]
                );
                for item in value["attention"].as_array().into_iter().flatten() {
                    line.push_str(&format!(
                        "\n{} {}: {} vouchers {}",
                        item["severity"], item["code"], item["message"], item["vouchers"]
                    ));
                }
                line
            }
            FileVatCmd::CompleteManual { .. } => format!(
                "VAT filing recorded for {} via {} (evidence {})",
                value["period"],
                value["manual_filing"]["channel"],
                value["manual_filing"]["evidence_ref"]
            ),
            FileVatCmd::Status { .. } => format!(
                "VAT {} — {}: {}",
                value["period"],
                value["status"],
                value["message"].as_str().unwrap_or_default()
            ),
            _ => format!(
                "file-vat run {} — {} (net_vat {}, bank_payment_required {})",
                value["run_id"], value["status"], value["net_vat"], value["bank_payment_required"]
            ),
        },
        Command::Employee { cmd } => match cmd {
            EmployeeCmd::List => format!("employees: {}", value["employees"]),
            EmployeeCmd::Add => format!("stored employee {}", value["employee_id"]),
            EmployeeCmd::Show { .. } => format!("employee {}", value["employee_id"]),
        },
        Command::FileWage {
            cmd: wage::WageCmd::EmployerReport(wage::EmployerReportCmd::Configure { .. }),
        } => format!(
            "employer_report {} (registration check: {})\nbefore: {}\nafter:  {}",
            if value["written"] == true {
                "written"
            } else if value["changed"] == false {
                "unchanged, nothing written"
            } else {
                "dry run, nothing written"
            },
            value["registration_verdict"].as_str().unwrap_or("?"),
            value["before"],
            value["after"]
        ),
        Command::FileWage { .. } => {
            let line = format!(
                "file-wage {}",
                value
                    .get("run_id")
                    .or_else(|| value.get("record_hash"))
                    .or_else(|| value.get("state"))
                    .or_else(|| value.get("comparison"))
                    .or_else(|| value.get("artifact"))
                    .and_then(Value::as_str)
                    .unwrap_or("dry-run")
            );
            match value["sensitive"]["record_hash"].as_str() {
                Some(note) => format!("{line}\nrecord_hash is sensitive: {note}"),
                None => line,
            }
        }
        Command::RunPayroll { .. } => format!(
            "run-payroll {} — {} [{}]",
            value["run_id"], value["status"], value["period"]
        ),
        Command::CaptureReceipt { cmd } => match cmd {
            CaptureReceiptCmd::Record { dry_run: true, .. } => format!(
                "receipt projection valid — preparation_hash {} (dry-run, no write)",
                value["preparation_hash"]
            ),
            CaptureReceiptCmd::Record { .. } => format!(
                "sealed receipt voucher #{} (replayed={})",
                value["number"], value["replayed"]
            ),
            _ => format!(
                "capture-receipt {} — {} [{}]",
                value["run_id"], value["status"], value["period"]
            ),
        },
        Command::HuntReceipts { .. } => format!(
            "hunt-receipts {} — {} [{}]",
            value["run_id"], value["status"], value["period"]
        ),
        Command::Obligation { cmd } => match cmd {
            ObligationCmd::List => format!(
                "obligations: {}",
                value["obligations"]
                    .as_array()
                    .map(|a| a.len())
                    .unwrap_or(0)
            ),
            ObligationCmd::Resolve { .. } => {
                format!("obligation {} — {}", value["id"], value["status"])
            }
        },
        Command::Knowledge { cmd } => knowledge::human_line(cmd, value),
        Command::Home { cmd } => home::human_line(cmd, value),
        Command::Intake { .. } => format!(
            "intake candidates: {}",
            value["candidates"].as_array().map(|a| a.len()).unwrap_or(0)
        ),
        // The gate content is what the user approves, so the human output shows it
        // in full rather than only summarising the run.
        Command::CloseYear {
            cmd: CloseYearCmd::Show { .. },
        } => {
            let line = format!(
                "close-year run {} — {} [{}]; open gate: {}",
                value["run_id"],
                value["status"],
                value["period"],
                value["open_gate"]["gate_id"].as_str().unwrap_or("none")
            );
            let line = match &value["open_gate"] {
                Value::Null => line,
                gate => format!(
                    "{line}\n{}",
                    serde_json::to_string_pretty(gate).expect("gate view serializes")
                ),
            };
            format!("{line}\n{}", close_year_next_human(value))
        }
        Command::CloseYear {
            cmd: CloseYearCmd::Start { dry_run: true },
        } => format!(
            "{}\n{}",
            value["message"].as_str().unwrap_or(""),
            serde_json::to_string_pretty(value).expect("preview serializes")
        ),
        Command::CloseYear { .. } => format!(
            "close-year run {} — {} [{}]\n{}",
            value["run_id"],
            value["status"],
            value["period"],
            close_year_next_human(value)
        ),
        Command::Report { .. } => reports::human_line(value),
        Command::FiscalYear { cmd } => match cmd {
            FiscalYearCmd::Show { .. } => format!(
                "fiscal year {} ({}..{}, {} months) — {} (statements_hash {})",
                value["year"],
                value["start"].as_str().unwrap_or("?"),
                value["end"].as_str().unwrap_or("?"),
                value["months"],
                value["state"],
                value["statements_hash"]
            ),
            FiscalYearCmd::Define { .. } => format!(
                "fiscal year {} defined: {}..{} ({} months), next year starts {}{}",
                value["year"],
                value["start"].as_str().unwrap_or("?"),
                value["end"].as_str().unwrap_or("?"),
                value["months"],
                value["next_start"].as_str().unwrap_or("?"),
                if value["replayed"] == true {
                    " (already defined)"
                } else {
                    ""
                }
            ),
            FiscalYearCmd::List => {
                let definitions = value["definitions"].as_array().cloned().unwrap_or_default();
                if definitions.is_empty() {
                    "fiscal years are calendar years (no definitions)".to_string()
                } else {
                    definitions
                        .iter()
                        .map(|d| {
                            format!(
                                "fiscal year {}: {}..{} ({} months)",
                                d["year"],
                                d["start"].as_str().unwrap_or("?"),
                                d["end"].as_str().unwrap_or("?"),
                                d["months"]
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            }
        },
        Command::Import { cmd } => match cmd {
            ImportCmd::Camt { cmd } => match cmd {
                CamtCmd::Parse { .. } => with_empty_report(
                    format!(
                        "parsed {} — {} transactions",
                        value["account_iban"],
                        value["transactions"]
                            .as_array()
                            .map(|a| a.len())
                            .unwrap_or(0)
                    ),
                    value,
                ),
                CamtCmd::Seal { dry_run: true, .. } => with_empty_report(
                    format!(
                        "would seal {} new vouchers from {}, {} transactions already in the \
                         books — preparation_hash {} (dry-run, no write)",
                        value["new_voucher_count"],
                        value["account_iban"],
                        value["duplicate_observation_count"],
                        value["preparation_hash"]
                    ),
                    value,
                ),
                CamtCmd::Seal { .. } => with_empty_report(
                    format!(
                        "sealed {} new vouchers from {}, {} transactions already in the books \
                         (attachment {})",
                        value["new_voucher_count"],
                        value["account_iban"],
                        value["duplicate_observation_count"],
                        value["attachment_ref"]
                    ),
                    value,
                ),
                CamtCmd::Observations { .. } => value["observations"]
                    .as_array()
                    .map(|rows| {
                        rows.iter()
                            .map(|o| {
                                format!(
                                    "{} {} #{}",
                                    o["attachment_ref"].as_str().unwrap_or_default(),
                                    o["event_key"].as_str().unwrap_or_default(),
                                    o["voucher_number"]
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default(),
            },
        },
    }
}

/// Append a camt import's empty-report message, when it has one, on its own
/// line so a report with no booked entries never reads as a plain success.
fn with_empty_report(line: String, value: &Value) -> String {
    match value["empty_report"]["message"].as_str() {
        Some(message) => format!("{line}\nempty report: {message}"),
        None => line,
    }
}

fn write_stdout_line(line: &str) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    writeln!(stdout, "{line}")
}

fn write_stdout_bytes(bytes: &[u8]) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(bytes)
}

fn write_stderr_line(line: &str) -> std::io::Result<()> {
    let stderr = std::io::stderr();
    let mut stderr = stderr.lock();
    writeln!(stderr, "{line}")
}

fn emit_error(error: &CliError) -> std::io::Result<()> {
    let mut payload = json!({
        "error_code": error.code,
        "error_kind": error.class.as_str(),
        "message": error.message,
    });
    // The user guide's Finnish "what to do" for codes it explains;
    // `aggountant explain <CODE>` prints the whole entry.
    if let Some(hint) = explain::hint(&error.code) {
        payload["hint"] = Value::String(hint);
    }
    if let Some(details) = &error.details {
        payload["details"] = details.clone();
    }
    if let Some(marker) = test_seam_marker() {
        payload["test_seam"] = marker;
    }
    write_stderr_line(&serde_json::to_string(&payload).expect("error payload serializes"))
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let is_display = matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            );
            if let Err(print_error) = error.print() {
                let _ = write_stderr_line(&format!(
                    "aggountant: could not print parser diagnostic: {print_error}"
                ));
                return failure_exit(ErrorClass::System);
            }
            return if is_display {
                ExitCode::SUCCESS
            } else {
                // clap defaults usage failures to 2. Canon §2 reserves 2 for
                // system/internal errors, so all genuine parser failures map to 1.
                failure_exit(ErrorClass::CallerActionable)
            };
        }
    };
    match run(&cli) {
        Ok(mut value) => {
            // A filing made through the test seam must say so wherever it is read:
            // as a field in JSON output, and as a notice on stderr for a person.
            if let Some(marker) = test_seam_marker() {
                if let Some(object) = value.as_object_mut() {
                    object.insert("test_seam".into(), marker);
                }
                if !cli.json {
                    let _ = write_stderr_line(&format!("warning: {TEST_SEAM_MESSAGE}"));
                }
            }
            let raw_skill = !cli.json
                && matches!(
                    &cli.command,
                    Command::Skill {
                        cmd: SkillCmd::Print { .. }
                    }
                );
            let report_document = match &cli.command {
                Command::Report { .. } if !cli.json => reports::stdout_content(&value),
                _ => None,
            };
            let write_result = if raw_skill {
                write_stdout_bytes(value["content"].as_str().unwrap_or("").as_bytes())
            } else if let Some(document) = report_document {
                write_stdout_bytes(document.as_bytes())
            } else {
                let output = if cli.json {
                    serde_json::to_string(&value).expect("output value serializes")
                } else {
                    human_line(&cli.command, &value)
                };
                write_stdout_line(&output)
            };
            match write_result {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    let error = CliError::system(
                        "IO_ERROR",
                        format!("could not write command output: {error}"),
                    );
                    let _ = emit_error(&error);
                    failure_exit(error.class)
                }
            }
        }
        Err(error) => {
            if emit_error(&error).is_err() {
                failure_exit(ErrorClass::System)
            } else {
                failure_exit(error.class)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        non_blank_path, perform_test_verification, read_config, resolve_file_vat_input,
        CliFileVatInput,
    };
    use aggountant_broker::{
        evaluate_vero_readiness, read_vero_profile, VeroAttestations, VeroFilingMode,
        VeroFilingProfile, VeroReadinessPurpose,
    };
    use aggountant_model::{BusinessId, FilingContactName, FilingContactPhone};
    use aggountant_vero_vat_api_mock::{
        MockVeroVatServer, MtlsTestCredentials, Scenario, GET_VAT_PERIODS_PATH,
    };
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn configuration_file_accepts_only_declared_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"home":"/data/books"}"#).unwrap();
        assert_eq!(
            read_config(&path).unwrap().home.unwrap(),
            Path::new("/data/books")
        );

        std::fs::write(&path, r#"{"home":"/data/books","unknown":true}"#).unwrap();
        assert_eq!(read_config(&path).unwrap_err().code, "INVALID_CONFIG");
    }

    #[test]
    fn blank_home_is_rejected() {
        assert_eq!(
            non_blank_path(Path::new("   "), "--home").unwrap_err().code,
            "INVALID_HOME"
        );
    }

    #[test]
    fn file_vat_input_uses_the_supplied_ready_profile_without_re_resolving_identity() {
        let input: CliFileVatInput = serde_json::from_value(json!({
            "period": "2026-Q1",
            "sales_vat": {"amount": "25.50", "currency": "EUR"},
            "purchase_vat": {"amount": "0.00", "currency": "EUR"},
            "payee_iban": "FI2112345600000785"
        }))
        .unwrap();
        let profile = VeroFilingProfile {
            schema_version: 1,
            mode: VeroFilingMode::Production,
            business_id: Some(BusinessId::parse("1572860-0").unwrap()),
            contact_name: Some(FilingContactName::parse("Ada Operator").unwrap()),
            contact_phone: Some(FilingContactPhone::parse("+358401234567").unwrap()),
            attestations: VeroAttestations::default(),
        };

        let resolved = resolve_file_vat_input(input, Some(&profile)).unwrap();
        assert_eq!(resolved.business_id.as_str(), "1572860-0");
        assert_eq!(resolved.contact_name.as_str(), "Ada Operator");
        assert_eq!(resolved.contact_phone.as_str(), "+358401234567");
    }

    #[test]
    fn explicit_test_verification_uses_mtls_read_and_persists_only_evidence() {
        let home = tempfile::tempdir().unwrap();
        let secrets = tempfile::tempdir().unwrap();
        let credential_path = secrets.path().join("vero-test.json");
        std::fs::write(
            &credential_path,
            json!({
                "target": "test",
                "business_id": "1572860-0",
                "software_key": "must-not-be-persisted",
                "client_certificate_pem": String::from_utf8_lossy(MtlsTestCredentials::CLIENT_CERTIFICATE_PEM),
                "client_private_key_pem": String::from_utf8_lossy(MtlsTestCredentials::CLIENT_PRIVATE_KEY_PEM),
                "ca_certificate_pem": String::from_utf8_lossy(MtlsTestCredentials::CA_CERTIFICATE_PEM),
            })
            .to_string(),
        )
        .unwrap();
        let mut profile = VeroFilingProfile {
            schema_version: 1,
            mode: VeroFilingMode::Disabled,
            business_id: Some(BusinessId::parse("1572860-0").unwrap()),
            contact_name: Some(FilingContactName::parse("Ada Operator").unwrap()),
            contact_phone: Some(FilingContactPhone::parse("+358401234567").unwrap()),
            attestations: VeroAttestations::default(),
        };
        let observed_at = aggountant_model::Instant::parse("2026-09-13T12:00:00Z").unwrap();
        let report = evaluate_vero_readiness(
            home.path(),
            Ok(profile.clone()),
            Some(&credential_path),
            VeroReadinessPurpose::TestVerification,
            observed_at,
        );
        assert!(report.ready);
        let server = MockVeroVatServer::start_mtls(Scenario::file_receipt("verify-only")).unwrap();
        let output = perform_test_verification(
            home.path(),
            &credential_path,
            &mut profile,
            report,
            &format!("https://localhost:{}/", server.addr().port()),
            observed_at,
        )
        .unwrap();
        assert_eq!(output["verified"], true);
        assert_eq!(output["mode"], "disabled");
        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].path, GET_VAT_PERIODS_PATH);
        // FilingYear is the observation's UTC year as a string, and the evidence
        // instant is written in serde form, both as before.
        assert_eq!(
            requests[0].json.as_ref().unwrap()["FilingYear"],
            json!("2026")
        );
        assert_eq!(output["observed_at"], "2026-09-13T12:00:00Z");
        let stored = std::fs::read_to_string(home.path().join("vero-filing.json")).unwrap();
        assert!(
            stored.contains("\"observed_at\": \"2026-09-13T12:00:00Z\""),
            "{stored}"
        );
        assert!(!stored.contains("must-not-be-persisted"));
        assert!(!stored.contains("PRIVATE KEY"));
        assert!(read_vero_profile(home.path())
            .unwrap()
            .attestations
            .test_verification
            .is_some());
    }
}
