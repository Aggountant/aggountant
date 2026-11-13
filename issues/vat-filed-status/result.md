# Result: vat-filed-status

`aggountant file-vat status --period <YYYY-MM|YYYY-Qn> [--json]` reports what
happened to a period's VAT return without sending anything.

| `status` | When | Asks Vero |
|---|---|---|
| `received` | receipt stored, GetFiledVATReturn says `Missing` | yes |
| `pending_authority_processing` | `Being Processed`, or `Processed` with a SubmittedDate before the receipt date (an earlier return) | yes |
| `processed` | `Processed`, submitted on or after the receipt date, values match the approved request | yes |
| `processed_values_differ` | `Processed` on or after the receipt date with different values | yes |
| `unrecognized_remote_status` | any other remote status, or a processed return without a comparable date | yes |
| `filed_without_receipt` | manual filing, or filing recorded by `file-vat reconcile` | no |
| `outcome_unknown` | a filing journalled `OUTCOME_UNKNOWN`, `MALFORMED…` or `HTTP_2…`; `next_command` is `file-vat reconcile --run-id …` | no |
| `filing_interrupted` | filing effect `Executing` (or `Succeeded` without the fiscal record) | no |
| `not_filed` | none of the above | no |

A failed read (network, authentication, malformed response) is
`VAT_STATUS_UNKNOWN`, `error_kind` system, exit 2. A broker that cannot be built
(readiness) is also `VAT_STATUS_UNKNOWN`, with the original code in the message
and exit 1. Every message about a receipted filing repeats that the receipt
proves it.

Acceptance criteria:

- Read-only status for a completed Run: done (`crates/process/src/file_vat/status.rs`, CLI `file-vat status`).
- No submission, proven with the mock: `crates/process/tests/file_vat_status.rs`
  runs against the mTLS `MockVeroVatServer` and asserts that the FileVATReturn
  request count is unchanged and that the home is byte-identical across
  `status`. The CLI tests in `crates/cli/tests/file_vat_status_cli.rs` give the
  scripted seam no FileVATReturn step.
- User guide fi+en: the "Ilmoitus ei näy OmaVerossa" / "The return does not
  show in OmaVero" troubleshooting section describes the command and its
  statuses and repeats that the acknowledgement, not OmaVero visibility,
  proves the filing. `VAT_STATUS_UNKNOWN` was added to error codes, and the
  command to the direct-filing task and the commands map. `skills/vero-fi`
  previously said no such command existed; it now describes it.

Gates: `scripts/ci-local.sh` green (fmt, clippy, cargo-deny, stable + MSRV build
and test, user-guide check). Review: `review.md` and `assessment.md` beside
this file.

## Decisions for later review

- **Selector is `--period`, not `--run-id`.** The question a user asks is about
  a month, and "not filed" and "filed manually" have no Run. The output names
  the Run when there is one. Alternative: `--run-id` like `reconcile`, or both;
  both would have meant two lookup paths.
- **Local answers exit 0 with a `status`; only a failed read is an error.**
  `not_filed`, `outcome_unknown` and the others are answers to the question,
  not failures. A non-zero exit is reserved for "could not determine", so a
  script that checks exit codes can never read an uncertain state as a clean
  answer. Alternative: refuse non-receipted cases with error codes.
- **The fetched status is not recorded.** The journal records what a Run did,
  and a completed Run has nothing left to do. Writing would also make the read
  unsafe to repeat freely. Alternative: an observation event on the Run.
- **Run lookup uses the journals, not the outbox.** The outbox is gitignored,
  so it is missing from a fresh clone of the books. The journals record the
  filing effect and its rejection code.
- **`MALFORMED` and `HTTP_2xx` filing rejections count as outcome unknown** in
  status (review F3). The filing path still treats them as definitive; see
  intake `staggeringly-aspiring-sky`.
- **Several Runs with a succeeded filing effect** count as one when they bound
  identical bytes. Otherwise status fails closed. Exact selection would need
  the ledger's filing `sent_hash` exposed as a read, and `crates/ledger` was
  outside this task's boundary.
- **The no-activity matcher fix also tightens `reconcile`.** A no-activity
  filing is no longer adopted when the remote return shows non-zero amounts.
  This direction is safe: such a Run stays `NeedsAttention`.
- **`Ledger::open` may apply a pending schema migration** in `status`, as it
  does in every other command including `preview`. No filing state changes.

## Found outside scope (intake, unlaned)

- `staggeringly-aspiring-sky`: a FileVATReturn HTTP 200 without a readable
  receipt is recorded as a definitive rejection, which permits a second
  filing.
- `perfectly-uppity-knife`: interrupted `file-vat` Runs have no CLI resume path
  (`resume_file_vat` is library-only), so `filing_interrupted` has no
  `next_command`.
