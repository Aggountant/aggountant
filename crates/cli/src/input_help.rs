//! The INPUT section of every command that reads JSON on stdin.
//!
//! Each constant is the `after_help` of one command: the stdin shape with field
//! meanings, which fields are required, and exactly one synthetic example in a
//! fenced `json` block. The help is the one description of the shape; skills
//! point here instead of keeping a copy. The tests below walk the command tree
//! so a stdin command without an INPUT section, or an example its command would
//! refuse to parse, fails the build.

pub(crate) const VOUCHER_DRAFT: &str = r#"INPUT (JSON on stdin)
  Required draft fields: document_date and posting_date (ISO YYYY-MM-DD), a
  non-empty description, and postings (at least two balanced entries).
  Each posting requires account, amount, and side. amount is exactly
  {"amount":"decimal string","currency":"EUR"}; side is "debit" or "credit".
  Account IDs must come from the active chart; after initializing the example
  home, inspect them with:
    aggountant --home ./accounting --json chart list
  Optional draft fields: attachment_ref (singular sha256 returned for an existing
  file by `aggountant --home ./accounting --json artifact attach --file ./receipt.pdf`),
  buyer_vat_id.
  Each posting may also carry optional vat_code. Omit optional fields when absent.

Example JSON (input-shape demonstration only, not VAT or classification advice):
```json
{"document_date":"2026-01-15","posting_date":"2026-01-15","description":"Fictional service transaction","postings":[{"account":"7690","amount":{"amount":"125.00","currency":"EUR"},"side":"debit"},{"account":"1910","amount":{"amount":"125.00","currency":"EUR"},"side":"credit"}]}
```
Run the printed example after selecting account IDs from the active chart:
  aggountant --home ./accounting --json voucher draft < draft.json"#;

pub(crate) const VOUCHER_VALIDATE: &str = r#"INPUT (JSON on stdin), one of two shapes
  1. The `voucher seal` envelope: {"draft":{...},"natural_key":"...","actor":"..."}.
     Validate the same file you seal. It is read exactly as `voucher seal` reads
     it (see `voucher seal --help`) and checked as seal checks it, including the
     fiscal calendar and period state (PERIOD_CLOSING, PERIOD_SEALED). A
     natural_key that already sealed this same content gives the warning
     NATURAL_KEY_ALREADY_SEALED (seal will replay that voucher); a natural_key
     bound to different content is refused with IDEMPOTENCY_KEY_REUSE, as seal
     would refuse it.
  2. A bare draft, the object under "draft" above (see `voucher draft --help`).
     Checked against the chart and artifacts only, not period state.
  Both shapes check that every posting account exists in the active chart, VAT
  rate-class mistags, and that a referenced attachment is stored. Warnings do
  not stop a seal; each carries a code and a message.

Example JSON (input-shape demonstration only, not VAT or classification advice):
```json
{"draft":{"document_date":"2026-01-15","posting_date":"2026-01-15","description":"Fictional service transaction","postings":[{"account":"7690","amount":{"amount":"125.00","currency":"EUR"},"side":"debit"},{"account":"1910","amount":{"amount":"125.00","currency":"EUR"},"side":"credit"}]},"natural_key":"fictional-service-2026-01-15","actor":"example-agent"}
```
Run it after `aggountant --home ./accounting --json init`, then seal the same file:
  aggountant --home ./accounting --json voucher validate < seal.json
  aggountant --home ./accounting --json voucher seal < seal.json"#;

pub(crate) const VOUCHER_SEAL: &str = r#"INPUT (JSON on stdin)
  Required fields: draft (the complete draft-input object described below) and
  natural_key (a non-empty caller-chosen idempotency key). Optional actor defaults
  to "cli". Do not add ledger-output fields or a policy_set field.
  Draft fields: document_date and posting_date (ISO YYYY-MM-DD), non-empty
  description, and at least two balanced postings. Each posting requires account,
  amount {"amount":"decimal string","currency":"EUR"}, and side "debit" or
  "credit". Account IDs must come from the active chart; after initializing the
  example home, inspect them with:
    aggountant --home ./accounting --json chart list
  Optional draft fields: attachment_ref (singular sha256 returned for an existing
  file by `aggountant --home ./accounting --json artifact attach --file ./receipt.pdf`),
  buyer_vat_id.
  Each posting may also carry optional vat_code. Omit optional fields when absent.

Example JSON (input-shape demonstration only, not VAT or classification advice):
```json
{"draft":{"document_date":"2026-01-15","posting_date":"2026-01-15","description":"Fictional service transaction","postings":[{"account":"7690","amount":{"amount":"125.00","currency":"EUR"},"side":"debit"},{"account":"1910","amount":{"amount":"125.00","currency":"EUR"},"side":"credit"}]},"natural_key":"fictional-service-2026-01-15","actor":"example-agent"}
```
Run the printed example after `aggountant --home ./accounting --json init` and
after selecting account IDs from the active chart. Dry-run the same file with
`voucher validate` first; it accepts this envelope unchanged:
  aggountant --home ./accounting --json voucher validate < seal.json
  aggountant --home ./accounting --json voucher seal < seal.json"#;

pub(crate) const CAPTURE_RECEIPT_RECORD: &str = r#"INPUT (JSON on stdin)
  Facts read from the receipt go under source; the agent's proposed treatment
  goes under proposal. The preparation hash binds both separately. Unknown
  fields are refused.
  natural_key     required. A stable key derived from the source. The same key
                  and content again replays the sealed voucher (replayed: true);
                  the same key with other content is refused.
  actor           optional, default "cli". Who prepared the entry.
  source          required object (a missing fact is CAPTURE_FACT_REQUIRED):
    amount          required, {"amount":"decimal string","currency":"EUR"}.
                    Gross for domestic-input; net for reverse-charge and no-vat.
    document_date   required, YYYY-MM-DD. The receipt or invoice date, not the
                    bank date; it selects the dated VAT rate.
    description     required, non-empty.
    attachment_ref  optional. The sha256 `artifact attach --file <file>` returned;
                    attach the source first so the voucher carries its evidence.
  proposal        required object:
    posting_date    required, YYYY-MM-DD.
    net_account     required. The expense account (`chart list`).
    contra_account  required. How the purchase was paid: a payable when the
                    company account or card paid it (the bank line later clears
                    it), a short-term liability when the owner paid, cash when
                    paid from the till. Never the bank account itself.
    vat             required. An explicit claim, tagged by "type":
      {"type":"domestic-input","rate_class":"..."}
          deductible domestic input VAT; source.amount is gross.
      {"type":"reverse-charge","rate_class":"...","scope":"..."}
          scope is eu-services, eu-goods or non-eu-services; source.amount is
          net. The chart's VAT roles for the scope receive the legs.
      {"type":"no-vat"}
          only when VAT-free status is confirmed by the source or practice.
      {"type":"clarification-required","question":"..."}
          a deciding fact is missing: refused as CAPTURE_CLARIFICATION_REQUIRED.
      {"type":"unsupported","treatment":"...","reason":"..."}
          outside this path: refused as CAPTURE_UNSUPPORTED_TREATMENT.
    rate_class names a class of the dated VAT rate table, for example
    domestic-standard, domestic-reduced-1 or domestic-reduced-2; an unknown
    class is refused.

Example JSON (input-shape demonstration only, not VAT or classification advice):
```json
{
  "natural_key": "receipt:fictional-vendor-2026-01-15",
  "actor": "example-agent",
  "source": {
    "amount": {"amount": "125.50", "currency": "EUR"},
    "document_date": "2026-01-15",
    "description": "Fictional service purchase"
  },
  "proposal": {
    "posting_date": "2026-01-16",
    "net_account": "7690",
    "contra_account": "2700",
    "vat": {"type": "domestic-input", "rate_class": "domestic-standard"}
  }
}
```
Preview, get the user's approval, then record with the returned hash:
  aggountant --home ./accounting --json capture-receipt record --dry-run < receipt.json
  aggountant --home ./accounting --json capture-receipt record \
    --approved-preparation-hash HASH < receipt.json"#;

pub(crate) const CAPTURE_RECEIPT_START: &str = r#"INPUT (JSON on stdin)
  All fields are required unless marked optional; unknown fields are refused.
  period                  YYYY-MM or YYYY-Qn, the period the voucher belongs to.
  topic                   a coarse, non-sensitive consult tag.
  question                the question put to the consult.
  facts                   [{"key":"...","value":"..."}]: the allowlisted facts
                          disclosed to the consult; nothing else leaves.
  raw_source              the receipt text. Never disclosed; only its hash is
                          stamped into provenance.
  nature                  the agent's reading of the expense: fully-deductible,
                          apportionable, passenger-car-mixed-use or
                          reverse-charge-eu-service.
  receipt_vat             the receipt's VAT, {"amount":"...","currency":"EUR"}.
  claimed_business_share  the user's claimed business-use share as a canonical
                          decimal string between 0 and 1 (e.g. "1", "0.7"). A
                          claim; the law computes the deductible VAT.
  document_date           the receipt date, YYYY-MM-DD.
  obligation              optional {"kind","description","deadline":YYYY-MM-DD}:
                          a genuine open follow-up, opened when the voucher seals.
  knowledge_slug          slug of the knowledge candidate distilled from the consult.
  draft                   the proposed voucher draft (see `voucher draft --help`).
                          Its deductible VAT must equal the assessed amount or
                          sealing is refused (CAPTURE_VAT_MISMATCH).
  tool_results            optional [{"tool":"...","result":<any JSON>}]: scripted
                          local tool results, used in order.

Example JSON (input-shape demonstration only, not VAT or classification advice):
```json
{
  "period": "2026-03",
  "topic": "vat-deductibility",
  "question": "Is VAT fully deductible for this fictional business service?",
  "facts": [{"key": "business_use", "value": "full"}],
  "raw_source": "FICTIONAL SERVICE RECEIPT\nNET EUR 100.00\nVAT EUR 25.50\nTOTAL EUR 125.50",
  "nature": "fully-deductible",
  "receipt_vat": {"amount": "25.50", "currency": "EUR"},
  "claimed_business_share": "1",
  "document_date": "2026-02-28",
  "knowledge_slug": "fictional-service-vat",
  "draft": {
    "document_date": "2026-02-28", "posting_date": "2026-03-10",
    "description": "Fictional business service",
    "postings": [
      {"account": "7690", "amount": {"amount": "100.00", "currency": "EUR"}, "side": "debit"},
      {"account": "1763", "amount": {"amount": "25.50", "currency": "EUR"}, "side": "debit"},
      {"account": "1910", "amount": {"amount": "125.50", "currency": "EUR"}, "side": "credit"}
    ]
  }
}
```
Run it:
  aggountant --home ./accounting --json capture-receipt start < consult.json"#;

pub(crate) const HUNT_RECEIPTS_START: &str = r#"INPUT (JSON on stdin)
  {"transactions":[...]}: the bank transactions to hunt evidence for. Each
  transaction requires every field below; unknown fields are refused.
  bank_account     the bank account identifier the transaction is on.
  txn_id           the bank's transaction id; unique together with bank_account.
  counterparty     the counterparty name the pulls search for.
  amount           {"amount":"decimal string","currency":"EUR"}.
  date             the bank date, YYYY-MM-DD.
  evidence_status  "missing" is hunted; any other value is skipped. At least one
                   transaction must be missing.
  proposed_draft   the voucher draft to seal when evidence is found (see
                   `voucher draft --help`); its attachment_ref is filled in from
                   the found artifact.

Example JSON (input-shape demonstration only, not VAT or classification advice):
```json
{
  "transactions": [{
    "bank_account": "FI2112345600000785", "txn_id": "t1",
    "counterparty": "Example Vendor",
    "amount": {"amount": "124.00", "currency": "EUR"}, "date": "2026-03-14",
    "evidence_status": "missing",
    "proposed_draft": {
      "document_date": "2026-03-14", "posting_date": "2026-03-14",
      "description": "Fictional online purchase",
      "postings": [
        {"account": "7690", "amount": {"amount": "124.00", "currency": "EUR"}, "side": "debit"},
        {"account": "1910", "amount": {"amount": "124.00", "currency": "EUR"}, "side": "credit"}
      ]
    }
  }]
}
```
Run it:
  aggountant --home ./accounting --json hunt-receipts start < hunt.json"#;

pub(crate) const CLOSE_YEAR_START: &str = r#"INPUT (JSON on stdin)
  Unknown fields are refused.
  year         required. The calendar year the fiscal year ends in.
  assets       optional list of assets to depreciate (poistot); the Domain
               computes every amount. Each requires:
                 asset_id, asset_class (a class of the depreciation rate table:
                 koneet-ja-kalusto, rakennukset, aineettomat-oikeudet),
                 expense_account (debited), accumulated_account (credited),
                 acquisition_cost (straight-line base) and book_value_start
                 (reducing-balance base; equals the cost in the first year),
                 both {"amount":"...","currency":"EUR"}.
  accruals     optional list of accruals (jaksotukset). Each requires
               accrual_id, description, debit_account, credit_account, amount.
  corrections  optional list of pre-close reclassifications, each sealed on its
               own approval (`close-year seal-correction`). Each requires
               correction_id, description, debit_account, credit_account, amount.

Example JSON (input-shape demonstration only, not accounting advice):
```json
{
  "year": 2026,
  "assets": [{
    "asset_id": "machine-1", "asset_class": "koneet-ja-kalusto",
    "expense_account": "6000", "accumulated_account": "1160",
    "acquisition_cost": {"amount": "10000.00", "currency": "EUR"},
    "book_value_start": {"amount": "8000.00", "currency": "EUR"}
  }],
  "accruals": [{
    "accrual_id": "electricity", "description": "Fictional electricity accrual",
    "debit_account": "7010", "credit_account": "2760",
    "amount": {"amount": "300.00", "currency": "EUR"}
  }],
  "corrections": []
}
```
Run it:
  aggountant --home ./accounting --json close-year start < close-year.json"#;

pub(crate) const EMPLOYEE_ADD: &str = r#"INPUT (JSON on stdin)
  Every field is required; unknown fields are refused. The record replaces any
  earlier one with the same employee_id. Personal data lands only in the
  encrypted store.
  employee_id  stable non-PII id used by run-payroll and file-wage.
  name         the employee's name.
  hetu         Finnish personal identity code; checked when a payroll Run starts.
  iban         payment account.
  tax_card     {"withholding_pct":"decimal string"}: the effective withholding
               percentage, 0..100.

Example JSON (input-shape demonstration only; fictional person):
```json
{"employee_id":"employee-1","name":"Maija Meikäläinen","hetu":"131052-308T","iban":"FI2112345600000785","tax_card":{"withholding_pct":"20.00"}}
```
Run it:
  aggountant --home ./accounting --json employee add < employee.json"#;

pub(crate) const RUN_PAYROLL_START: &str = r#"INPUT (JSON on stdin)
  Only the gross pay is given; every statutory amount is computed. Every field
  is required; unknown fields are refused.
  payment_date  YYYY-MM-DD. Drives the rate edition and the Incomes Register
                deadline.
  employees     list of {employee_id, gross, tulolaji}:
    employee_id   an id stored with `employee add`.
    gross         {"amount":"...","currency":"EUR"}, positive, whole cents.
    tulolaji      an Incomes Register income type code supported for payroll,
                  for example "201" (time-rate pay).

Example JSON (input-shape demonstration only, not payroll advice):
```json
{"payment_date":"2026-02-27","employees":[{"employee_id":"employee-1","gross":{"amount":"3000.00","currency":"EUR"},"tulolaji":"201"}]}
```
Run it:
  aggountant --home ./accounting --json run-payroll start < payroll.json"#;

pub(crate) const IMPORT_CAMT053_SEAL: &str = r#"INPUT (JSON on stdin)
  {"assignments":[...],"actor":"..."}. One assignment for every booked entry of
  the statement (`import camt053 parse` lists them with their bank_ref); an
  entry without one refuses the whole batch (MISSING_ASSIGNMENT). Unknown fields
  are refused.
  actor             optional, default "cli".
  Each assignment:
    bank_ref         required. The entry's bank_ref from `parse`.
    bank_account     required. The ledger account of the bank leg, e.g. 1910.
    contra_postings  required, non-empty list of {account, amount, vat_code?}.
                     Amounts are gross and sum exactly to the entry's amount.
                     vat_code is optional and only a zero-rate code is accepted.
    vat_treatment    optional; purchase (money-out) entries only, with exactly
                     one contra posting. Expands the VAT legs:
                       {"type":"domestic-input","rate_class":"..."} (gross)
                       {"type":"reverse-charge","rate_class":"...","scope":
                        "eu-services|eu-goods|non-eu-services"} (net)
                       {"type":"no-vat"}
                     and the -partial variants of the first two with
                       "deductibility":{"numerator":N,"denominator":D,
                        "provenance":{"source":"documented-use-allocation",
                        "reference":"..."}}  (0 < N < D).
                     Sales VAT is not expanded here; do not import such an entry.
    document_date    YYYY-MM-DD, the source document's date. Required with
                     vat_treatment (it selects the VAT rate; the bank date stays
                     the posting date). Otherwise optional.
    description      optional; defaults to counterparty and remittance info.
  A payment whose receipt is recorded with `capture-receipt record` against a
  payable is assigned to that payable here, without VAT.

Example JSON (input-shape demonstration only, not VAT or classification advice):
```json
{
  "actor": "example-agent",
  "assignments": [
    {"bank_ref": "BANKREF-0001", "bank_account": "1910",
     "document_date": "2026-03-14",
     "contra_postings": [{"account": "7690", "amount": {"amount": "124.00", "currency": "EUR"}}],
     "vat_treatment": {"type": "domestic-input", "rate_class": "domestic-standard"}},
    {"bank_ref": "BANKREF-0002", "bank_account": "1910",
     "contra_postings": [{"account": "2700", "amount": {"amount": "62.00", "currency": "EUR"}}]}
  ]
}
```
Run it:
  aggountant --home ./accounting --json import camt053 seal --file statement.xml < assignments.json"#;

pub(crate) const VERO_FI_CONFIGURE: &str = r#"INPUT (JSON on stdin)
  The whole vero-filing.json profile; it replaces the stored profile, it is not
  merged. Carry over the attestation evidence blocks from the current profile
  (its path is `vero-fi status`'s profile_path): leaving out
  attestations.portal_verification drops the evidence and blocks production.
  Unknown fields are refused.
  schema_version  required, 1.
  mode            required, "disabled" or "production". Production is written
                  only when readiness is ready (otherwise VERO_NOT_READY).
  business_id     optional, Business ID 1234567-8 with a valid check digit.
  contact_name    optional, 1..70 characters.
  contact_phone   optional, 5..15 digits with phone punctuation (+ - space ()).
  attestations    optional object, every member optional (false or absent):
                  software_registered, api_agreement_active,
                  production_verification_completed,
                  production_permission_active (booleans),
                  activation_certificate_fingerprint_sha256 (64 hex),
                  test_verification and portal_verification (evidence the
                  verify commands record; copy them unchanged).
  credential_path optional. Platform-local credential bundle path, stored in
                  the platform configuration, not in the profile.

Example JSON (input-shape demonstration only; fictional company):
```json
{
  "schema_version": 1,
  "mode": "disabled",
  "business_id": "1234567-1",
  "contact_name": "Maija Meikäläinen",
  "contact_phone": "+358 40 1234567",
  "attestations": {
    "software_registered": false,
    "api_agreement_active": false,
    "production_verification_completed": false,
    "production_permission_active": false
  }
}
```
Check first, then write:
  aggountant --home ./accounting --json vero-fi configure --dry-run < vero-filing.json
  aggountant --home ./accounting --json vero-fi configure < vero-filing.json"#;

pub(crate) const FILE_VAT_START: &str = concat!(
    r#"INPUT (JSON on stdin)
  Take the figures from `file-vat preview`; they are derived again from the
  sealed books and a difference is refused (VAT_FIGURES_MISMATCH). Unknown
  fields are refused.
  period         required, the VAT period (YYYY-MM or YYYY-Qn).
  sales_vat      required, {"amount":"...","currency":"EUR"}.
  purchase_vat   required, {"amount":"...","currency":"EUR"}.
  payee_iban     required. Goes only into the local payment proposal (the Tax
                 Administration account shown in OmaVero); nothing is paid.
  business_id, contact_name, contact_phone
                 optional; taken from vero-filing.json and refused when they
                 differ from it (VERO_FILING_PROFILE_MISMATCH). Required only
                 under the AGGOUNTANT_BROKER_SCRIPT test seam.
  vat_reconciliation_acknowledgement
                 optional {"reason":"...","vouchers":[N,...]}: file although
                 untagged VAT-account postings remain. vouchers must name
                 exactly the vouchers the preview lists, or the start is
                 refused (VAT_ACCOUNTS_UNRECONCILED).

Example JSON (input-shape demonstration only, not VAT advice):
```json
{"period":"2026-08","sales_vat":{"amount":"33.59","currency":"EUR"},"purchase_vat":{"amount":"45.68","currency":"EUR"},"payee_iban":"FI2112345600000785"}
```
Run it:
  aggountant --home ./accounting --json file-vat start < start.json

"#,
    vero_transport_help!()
);

pub(crate) const FILE_WAGE_REPORT: &str = r#"INPUT (JSON on stdin)
  One tax-free travel earnings report. The JSON carries no personal identity
  code: the earner is an employee_id stored with `employee add`. Every field is
  required; unknown fields are refused.
  business_id       the payer's Business ID (1234567-8 with a valid check digit).
  source_reference  the payer's own stable reference, 1..200 bytes.
  employee_id       an id stored with `employee add`.
  payment_date      YYYY-MM-DD, 2019-01-01 or later and at most 45 days ahead.
                    The report is due five days after it.
  period_start, period_end
                    YYYY-MM-DD, the earning period, given explicitly.
  joint_owner       true or false: joint ownership (IncomeEarnerType 2), stated
                    explicitly, not inferred from a role.
  contact_name      1..70 characters.
  contact_phone     5..15 digits with phone punctuation.
  lines             1..100 lines, each tagged by income_type; amount is a
                    positive decimal string in EUR cents:
                      {"income_type":"311","amount":"...","kilometers":N}
                          kilometre allowance, positive whole kilometres.
                      {"income_type":"331","amount":"...","allowance_code":N}
                          daily allowance: 1 meal, 2 partial, 3 full,
                          4 international.

Example JSON (input-shape demonstration only; fictional company and person):
```json
{
  "business_id": "1234567-1",
  "source_reference": "expense-payment-2026-09-001",
  "employee_id": "employee-1",
  "payment_date": "2026-10-25",
  "period_start": "2026-09-01",
  "period_end": "2026-09-30",
  "joint_owner": true,
  "contact_name": "Maija Meikäläinen",
  "contact_phone": "+358 40 1234567",
  "lines": [
    {"income_type": "311", "amount": "120.00", "kilometers": 218},
    {"income_type": "331", "amount": "54.00", "allowance_code": 3}
  ]
}
```
Preview, then start with the same timestamp:
  aggountant --home ./accounting --json file-wage preview --timestamp 2026-10-25T12:00:00Z < report.json
  aggountant --home ./accounting --json file-wage start --timestamp 2026-10-25T12:00:00Z < report.json"#;

pub(crate) const EMPLOYER_REPORT_CONFIGURE: &str = r#"INPUT (JSON on stdin, only when no field flag is given)
  The employer_report block of .aggountant/tulorekisteri.json, which must
  already exist. Every field is required; unknown fields are refused.
  registered_from     YYYY-MM, 2019-01 or later: the first month of the
                      employer-register entry. A report is due for every month
                      from it on.
  contact_name        1..70 characters.
  contact_phone       5..15 digits with phone punctuation.
  pension_insurances  list of {"provider_code":N,"policy_number":"..."}, at most
                      20; provider_code is the Incomes Register PensionProvIdCode
                      (1..99999), policy_number 1..11 characters. An empty list
                      states that there is no obligation to insure (for example
                      only YEL-insured owners).

Example JSON (input-shape demonstration only; fictional company):
```json
{"registered_from":"2020-02","contact_name":"Maija Meikäläinen","contact_phone":"+358 40 1234567","pension_insurances":[{"provider_code":46,"policy_number":"12-3456789"}]}
```
Run it:
  aggountant --home ./accounting --json file-wage employer-report configure --offline --dry-run < employer-report.json"#;

pub(crate) const KNOWLEDGE_COMPANY_ADD: &str = r#"INPUT (JSON on stdin)
  Every field is required; unknown fields are refused.
  id         lowercase kebab-case, at most 128 characters; must not exist yet.
  statement  the fact or practice, in prose.
  scope      prose: when it applies and its important limits.
  source     {"reference":"...","date":"YYYY-MM-DD"}: a human-readable pointer
             to the evidence or decision, and the date of that source (not
             today's date).

Example JSON (input-shape demonstration only):
```json
{"id":"bank-fees-practice","statement":"Bank service fees are booked to 8490.","scope":"All bank accounts.","source":{"reference":"user decision 2026-09-12","date":"2026-09-12"}}
```
Run it:
  aggountant --home ./accounting --json knowledge company add < fact.json"#;

pub(crate) const KNOWLEDGE_COMPANY_UPDATE: &str = r#"INPUT (JSON on stdin)
  The new revision. Unknown fields are refused.
  id         optional; see FACT ID.
  statement  required, the revised fact or practice.
  scope      required, the revised scope.
  source     required, {"reference":"...","date":"YYYY-MM-DD"}.
  reason     required, why the earlier understanding changed.

FACT ID
  Give the id positionally, with --id, or as "id" in the stdin JSON. When the
  JSON id and the argument are both given they must be the same id.

Example JSON (input-shape demonstration only):
```json
{"statement":"Bank service fees are booked to 8490.","scope":"All bank accounts.","source":{"reference":"user decision 2026-10-01","date":"2026-10-01"},"reason":"Scope widened."}
```
Run it:
  aggountant --home ./accounting --json knowledge company update bank-fees-practice < update.json"#;

pub(crate) const KNOWLEDGE_COMPANY_RETIRE: &str = r#"INPUT (JSON on stdin)
  Unknown fields are refused.
  id      optional; see FACT ID.
  source  required, {"reference":"...","date":"YYYY-MM-DD"}.
  reason  required, why the fact no longer holds.

FACT ID
  Give the id positionally, with --id, or as "id" in the stdin JSON. When the
  JSON id and the argument are both given they must be the same id.

Example JSON (input-shape demonstration only):
```json
{"source":{"reference":"user decision 2026-11-01","date":"2026-11-01"},"reason":"Bank account closed."}
```
Run it:
  aggountant --home ./accounting --json knowledge company retire bank-fees-practice < retire.json"#;

#[cfg(test)]
mod tests {
    use clap::CommandFactory;
    use serde::de::DeserializeOwned;

    use crate::Cli;

    /// Every command that reads JSON on stdin, with the type its handler parses
    /// the input into. A new stdin command belongs here and needs an INPUT
    /// section; the tree walk below fails otherwise.
    type ParseCheck = fn(&str) -> Result<(), String>;

    fn stdin_commands() -> Vec<(&'static str, ParseCheck)> {
        fn parses<T: DeserializeOwned>(json: &str) -> Result<(), String> {
            serde_json::from_str::<T>(json)
                .map(drop)
                .map_err(|e| e.to_string())
        }
        vec![
            ("voucher draft", parses::<aggountant_domain::DraftInput>),
            ("voucher validate", parses::<crate::SealInput>),
            ("voucher seal", parses::<crate::SealInput>),
            (
                "capture-receipt record",
                parses::<aggountant_process::StructuredReceiptInput>,
            ),
            (
                "capture-receipt start",
                parses::<aggountant_process::CaptureReceiptInput>,
            ),
            (
                "hunt-receipts start",
                parses::<aggountant_process::HuntReceiptsInput>,
            ),
            (
                "close-year start",
                parses::<aggountant_process::CloseYearInput>,
            ),
            ("employee add", parses::<aggountant_ledger::EmployeeState>),
            (
                "run-payroll start",
                parses::<aggountant_domain::PayrollIntent>,
            ),
            ("import camt053 seal", parses::<crate::ImportSealInput>),
            ("vero-fi configure", |json| {
                let input: crate::VeroConfigureInput =
                    serde_json::from_str(json).map_err(|e| e.to_string())?;
                input.profile().validate()
            }),
            ("file-vat start", parses::<crate::CliFileVatInput>),
            ("file-wage preview", parse_wage_report),
            ("file-wage start", parse_wage_report),
            ("file-wage employer-report configure", |json| {
                let profile: aggountant_broker::tulorekisteri::EmployerReportProfile =
                    serde_json::from_str(json).map_err(|e| e.to_string())?;
                profile.validate()
            }),
            (
                "knowledge company add",
                parses::<aggountant_process::AddCompanyKnowledge>,
            ),
            (
                "knowledge company update",
                parses::<crate::knowledge::UpdateInput>,
            ),
            (
                "knowledge company retire",
                parses::<crate::knowledge::RetireInput>,
            ),
        ]
    }

    fn parse_wage_report(json: &str) -> Result<(), String> {
        let report: aggountant_domain::WageReport =
            serde_json::from_str(json).map_err(|e| e.to_string())?;
        report
            .validate(aggountant_model::Date::parse("2026-10-25").unwrap().inner())
            .map_err(|e| e.to_string())
    }

    /// `(path, after_help)` for every leaf command.
    fn leaves(cmd: &clap::Command, prefix: &str, out: &mut Vec<(String, String)>) {
        for sub in cmd.get_subcommands() {
            let path = format!("{prefix}{}", sub.get_name()).trim().to_string();
            if sub.has_subcommands() {
                leaves(sub, &format!("{path} "), out);
            } else {
                let help = sub
                    .get_after_help()
                    .map(|h| h.to_string())
                    .unwrap_or_default();
                out.push((path, help));
            }
        }
    }

    fn only_json_example(help: &str) -> &str {
        let mut blocks = help.split("```json\n").skip(1);
        let block = blocks.next().expect("INPUT section has a JSON example");
        assert!(blocks.next().is_none(), "exactly one JSON example");
        block.split_once("\n```").expect("JSON example is closed").0
    }

    #[test]
    fn every_stdin_command_has_an_input_section_and_only_those_do() {
        let mut all = Vec::new();
        leaves(&Cli::command(), "", &mut all);
        let expected: Vec<&str> = stdin_commands().iter().map(|(p, _)| *p).collect();
        for path in &expected {
            let (_, help) = all
                .iter()
                .find(|(p, _)| p == path)
                .unwrap_or_else(|| panic!("no command {path:?}"));
            assert!(help.starts_with("INPUT (JSON on stdin"), "{path}: {help}");
        }
        for (path, help) in &all {
            if help.starts_with("INPUT") {
                assert!(
                    expected.contains(&path.as_str()),
                    "{path} has an INPUT section but is not listed as a stdin command"
                );
            }
        }
        // A command whose own description says it reads stdin is a stdin command.
        let cli = Cli::command();
        for (path, _) in &all {
            let mut cmd = &cli;
            for part in path.split(' ') {
                cmd = cmd.find_subcommand(part).expect("leaf path resolves");
            }
            let about = format!(
                "{} {}",
                cmd.get_about().map(|a| a.to_string()).unwrap_or_default(),
                cmd.get_long_about()
                    .map(|a| a.to_string())
                    .unwrap_or_default()
            );
            if about.contains("stdin") {
                assert!(
                    expected.contains(&path.as_str()),
                    "{path} mentions stdin but has no INPUT section"
                );
            }
        }
    }

    #[test]
    fn every_input_example_parses_as_its_command_reads_it() {
        let mut all = Vec::new();
        leaves(&Cli::command(), "", &mut all);
        for (path, check) in stdin_commands() {
            let help = &all.iter().find(|(p, _)| p == path).unwrap().1;
            let example = only_json_example(help);
            check(example).unwrap_or_else(|e| panic!("{path} example refused: {e}"));
            assert!(
                help.contains(&format!("aggountant --home ./accounting --json {path}")),
                "{path}: the help shows how to run the example"
            );
        }
    }
}
