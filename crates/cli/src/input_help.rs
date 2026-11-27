//! The INPUT section of every command that reads JSON on stdin.
//!
//! Each constant is the `after_help` of one command: the stdin shape with field
//! meanings, which fields are required, and exactly one synthetic example in a
//! fenced `json` block. The help is the one description of the shape; skills
//! point here instead of keeping a copy. The tests below walk the command tree
//! so a stdin command without an INPUT section, or an example its command would
//! refuse to parse, fails the build.

/// The `vat_code` paragraph shared by the draft, seal and validate help: one
/// description of where a manual voucher's VAT code goes and its grammar.
macro_rules! vat_code_input {
    () => {
        r#"  vat_code (optional, per posting) is how a manually prepared voucher reaches
  the VAT return. Put it on the VAT base line (the net sales or expense
  posting), never on the VAT account line, which carries no code: the VAT
  return derives each tagged posting's VAT at the rate in force on
  document_date and reconciles it against the VAT accounts, so a code on the
  VAT line would count VAT on VAT and an untagged base line is missing from
  the return. The derived VAT is round(base x rate / 100); for a margin code
  it is extracted from within the tagged margin, round(margin x rate /
  (100 + rate)). Book the VAT line at that derived amount (with -deduct<NN>,
  the deductible share of it).
  Grammar: <direction>-<rate class>. direction is sales (output VAT) or
  purchase (deductible input VAT); the rate class comes from the dated rate
  table, e.g. sales-domestic-standard, purchase-domestic-reduced-1,
  sales-eu-goods (needs buyer_vat_id). A purchase code takes a -deduct<NN>
  suffix (NN 1..99) for partial deduction: purchase-domestic-standard-deduct50.
  A margin-scheme sale is sales-margin-<used-goods|art|antiques|collectibles|
  vehicle>, tagged on the VAT-inclusive margin, not the price. Refused:
  purchase-eu-* (an EU acquisition is reverse-charged), -deduct on a sales
  code, purchase-margin-*. List the codes valid on a document date (offline):
    aggountant --json voucher vat-codes --document-date 2026-01-15
"#
    };
}

pub(crate) const VOUCHER_DRAFT: &str = concat!(
    r#"INPUT (JSON on stdin)
  Required draft fields: document_date and posting_date (ISO YYYY-MM-DD), a
  non-empty description, and postings (at least two balanced entries).
  Each posting requires account, amount, and side. amount is exactly
  {"amount":"decimal string","currency":"EUR"}; side is "debit" or "credit".
  Account IDs must come from the active chart; after initializing the example
  home, inspect them with:
    aggountant --home ./accounting --json chart list
  Optional draft fields: attachment_ref (singular sha256 returned for an existing
  file by `aggountant --home ./accounting --json artifact attach --file ./receipt.pdf`),
  buyer_vat_id, corrects_vat_period.
  corrects_vat_period (YYYY-MM or YYYY-Qn) marks the voucher as a correction to
  the already filed VAT return of an earlier period: its VAT is then counted in
  that period's replacement return (`file-vat replace`) and left out of the
  return of the period it is posted in. posting_date stays in an open period;
  keep document_date at the original document's date, since the VAT rate is
  resolved from it. Refused unless the named period is sealed and its return
  filed (VAT_PERIOD_NOT_FILED). Leave it out for a small error corrected in
  the current period's return.
  Omit optional fields when absent.
"#,
    vat_code_input!(),
    r#"

Example JSON (input-shape demonstration only, not VAT or classification advice):
```json
{"document_date":"2026-01-15","posting_date":"2026-01-15","description":"Fictional service transaction","postings":[{"account":"7690","amount":{"amount":"125.00","currency":"EUR"},"side":"debit"},{"account":"1910","amount":{"amount":"125.00","currency":"EUR"},"side":"credit"}]}
```
Run the printed example after selecting account IDs from the active chart:
  aggountant --home ./accounting --json voucher draft < draft.json"#
);

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

pub(crate) const VOUCHER_SEAL: &str = concat!(
    r#"INPUT (JSON on stdin)
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
  buyer_vat_id, corrects_vat_period.
  corrects_vat_period (YYYY-MM or YYYY-Qn) marks the voucher as a correction to
  the already filed VAT return of an earlier period: its VAT is then counted in
  that period's replacement return (`file-vat replace`) and left out of the
  return of the period it is posted in. posting_date stays in an open period;
  keep document_date at the original document's date, since the VAT rate is
  resolved from it. Refused unless the named period is sealed and its return
  filed (VAT_PERIOD_NOT_FILED). Leave it out for a small error corrected in
  the current period's return.
  Omit optional fields when absent.
"#,
    vat_code_input!(),
    r#"

Example JSON (input-shape demonstration only, not VAT or classification advice):
```json
{"draft":{"document_date":"2026-01-15","posting_date":"2026-01-15","description":"Fictional service transaction","postings":[{"account":"7690","amount":{"amount":"125.00","currency":"EUR"},"side":"debit"},{"account":"1910","amount":{"amount":"125.00","currency":"EUR"},"side":"credit"}]},"natural_key":"fictional-service-2026-01-15","actor":"example-agent"}
```
Run the printed example after `aggountant --home ./accounting --json init` and
after selecting account IDs from the active chart. Dry-run the same file with
`voucher validate` first; it accepts this envelope unchanged:
  aggountant --home ./accounting --json voucher validate < seal.json
  aggountant --home ./accounting --json voucher seal < seal.json"#
);

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
          It takes no other member: a rate_class or scope on it is refused.
      {"type":"clarification-required","question":"..."}
          a deciding fact is missing: refused as CAPTURE_CLARIFICATION_REQUIRED.
      {"type":"unsupported","treatment":"...","reason":"..."}
          outside this path: refused as CAPTURE_UNSUPPORTED_TREATMENT.
    rate_class names the Finnish rate that applies, a domestic class of the
    dated VAT rate table such as domestic-standard, domestic-reduced-1 or
    domestic-reduced-2, also for a reverse charge (scope says EU or non-EU).
    An unknown class is refused; the zero-rated sale classes eu-goods,
    eu-services and zero-rated are refused as VAT_TREATMENT_CLASS_MISMATCH,
    except for one booking: an intra-Community acquisition of goods that is
    exempt in Finland is
      {"type":"reverse-charge","rate_class":"zero-rated","scope":"eu-goods"}
    and files its value in the EU goods purchases with zero tax. Use it only
    when the exemption is confirmed: an invoice without VAT does not make the
    acquisition exempt, and a taxable one takes the domestic class.

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
                          Tag the VAT base posting with its vat_code (e.g.
                          purchase-domestic-standard) and leave the VAT-account
                          leg untagged: the VAT return counts only tagged VAT.
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
      {"account": "7690", "amount": {"amount": "100.00", "currency": "EUR"}, "side": "debit",
       "vat_code": "purchase-domestic-standard"},
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
  assets       optional list of assets to depreciate (poistot) that the previous
               sealed year does not carry; the Domain computes every amount.
               Each requires:
                 asset_id, asset_class (a class of the depreciation rate table:
                 koneet-ja-kalusto, rakennukset, aineettomat-oikeudet),
                 expense_account (debited), accumulated_account (credited),
                 acquisition_cost (straight-line base) and book_value_start
                 (reducing-balance base; equals the cost in the first year),
                 both {"amount":"...","currency":"EUR"}.
  removed_assets
               optional list of asset_ids that leave the register in this year
               (sold, scrapped, or depreciated by hand from now on). They get
               no depreciation from the run.
  accruals     optional list of accruals (jaksotukset). Each requires
               accrual_id, description, debit_account, credit_account, amount.
  corrections  optional list of entries for the closing year that the program
               does not calculate (a reclassification, an error found in the
               closed months, a depreciation calculated elsewhere). Each is
               sealed on its own approval (`close-year seal-correction`) before
               ApproveAdjustments is approved. Each requires
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
THE ASSET REGISTER
  Sealing a year stores its assets and their closing book values. The next
  year's close carries them from there: each opens at the value the sealed
  year closed it at, and is depreciated without being named in `assets`. Give
  in `assets` only what the register does not hold: new acquisitions, or, in
  the first close of a home that has no register yet, every asset with its
  book value at the start of the year. A carried asset given again must repeat
  the carried figures exactly; anything else is refused. To replace a carried
  asset's figures on purpose, name it in `removed_assets` and give it again.

DRY RUN
  `close-year start --dry-run` takes the same input and writes nothing. It
  shows `asset_register` (per asset: source carried|input, book_value_start,
  depreciation, book_value_end), `depreciation` (method and rate_percent per
  asset), `opening_register_year` (the sealed year the carried assets come
  from, null when there is none), `blockers` and `would_start`. `{"year": N}`
  alone shows what the register carries into year N.

Run it:
  aggountant --home ./accounting --json close-year start --dry-run < close-year.json
  aggountant --home ./accounting --json close-year start < close-year.json"#;

pub(crate) const EMPLOYEE_ADD: &str = r#"INPUT (JSON on stdin)
  Every field is required; unknown fields are refused. The record replaces any
  earlier one with the same employee_id. Personal data is kept only in the
  home's separate PII store (masterdata/pii.sqlite), which the home's
  .gitignore keeps out of git; it is not encrypted at rest in this version.
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

pub(crate) const IMPORT_CAMT_SEAL: &str = r#"INPUT (JSON on stdin)
  {"assignments":[...],"existing_vouchers":[...],"actor":"..."}. One assignment
  or one existing_vouchers binding (never both) for every booked entry of the
  statement (`import camt parse` lists them with their bank_ref); an entry
  without either refuses the whole batch (MISSING_ASSIGNMENT). Unknown
  top-level, assignment and binding fields are refused.
  actor             optional, default "cli".
  existing_vouchers optional list of {bank_ref, bank_account, voucher_number}:
                    entries already booked by a voucher sealed outside the
                    import (for example with `voucher seal`). The entry is
                    bound to that voucher instead of sealing a new one; see
                    VOUCHERS SEALED OUTSIDE THE IMPORT below.
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
                     A partial deduction needs the -partial type; a member
                     that does not belong to the type is refused. rate_class
                     is a domestic class (domestic-standard, ...), also for a
                     reverse charge; eu-goods, eu-services and zero-rated are
                     sale classes, refused as VAT_TREATMENT_CLASS_MISMATCH,
                     except an intra-Community acquisition of goods exempt in
                     Finland: {"type":"reverse-charge","rate_class":
                     "zero-rated","scope":"eu-goods"} (zero tax, value in the
                     EU goods purchases), only when the exemption is confirmed.
                     Sales VAT is not expanded here; do not import such an entry.
    document_date    YYYY-MM-DD, the source document's date. Required with
                     vat_treatment (it selects the VAT rate; the bank date stays
                     the posting date). Otherwise optional.
    description      optional; defaults to counterparty and remittance info.
    not_booked_by    optional list of voucher numbers you have checked do not
                     book this entry; silences BANK_ENTRY_POSSIBLY_BOOKED for
                     exactly those vouchers.
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
TRANSACTIONS ALREADY IN THE BOOKS
  A transaction's event key is its natural_key (bank reference and account), so
  the same transaction in an overlapping export is recognised. An assignment is
  still required for it, and it must be the one its voucher was sealed with.
  Every entry resolves to one outcome:
    new_voucher                    sealed now, with this file as attachment.
    duplicate_source_observation   same bank facts and same booking as voucher
                                   voucher_number: nothing is sealed; the file
                                   is recorded as a source it was seen in.
    bound_existing_voucher         bound to the existing voucher
                                   voucher_number: nothing is sealed.
  and any of these stops the whole batch before anything is written (user
  error; details.conflicts lists every conflicting entry with its voucher):
    SOURCE_FACTS_CHANGED           the bank's facts differ (amount, dates,
                                   direction, parties, remittance), or the file
                                   lists a booked transaction as not booked.
    POSTING_PROJECTION_CHANGED     same facts, different booking (accounts,
                                   amounts, VAT, dates or description).
    SOURCE_IDENTITY_UNKNOWN        a voucher holds the key but its transaction
                                   cannot be read back from its attachment.
    BANK_ENTRY_POSSIBLY_BOOKED     a voucher sealed outside the import may
                                   already book a new entry (below).
    BANK_VOUCHER_MISMATCH          a binding's voucher does not book the entry,
                                   is bound to another transaction, or the
                                   transaction is bound to another voucher.
  Outputs and conflict details carry new_voucher_count,
  duplicate_observation_count, bound_count, conflict_count and
  backfilled_count (vouchers sealed by an earlier release whose identity was
  read from their statement).
  `import camt observations` lists which transactions each file contained.

VOUCHERS SEALED OUTSIDE THE IMPORT
  A bank transaction booked with `voucher seal` has no event key, so before a
  new voucher is sealed for an entry the import searches every voucher not yet
  bound to a bank transaction. One whose natural key or description names the
  entry's bank_ref (evidence bank_reference), or that posts the entry's amount
  on the entry's side of its bank_account dated within 7 days of the booking or
  value date (evidence account_amount_date), stops the batch with
  BANK_ENTRY_POSSIBLY_BOOKED; details.conflicts names each voucher. Then either
  - the voucher books the entry: move the entry from assignments to
    existing_vouchers with that voucher_number. Its bank leg must match (same
    account, amount, side, and a date within 7 days) or the batch stops with
    BANK_VOUCHER_MISMATCH. The voucher is not changed; later exports of the
    entry need the same binding; or
  - it does not: add its number to the assignment's not_booked_by.
  A voucher is bound to at most one bank transaction, and a bound voucher
  stays a candidate for other entries (one voucher can book several
  payments). If it books another entry too, reverse that part of it with a
  correction voucher before the import books the entry, then list it in
  not_booked_by.

PREVIEW AND APPROVAL
  --dry-run prepares and validates every voucher exactly as the seal would
  (accounts, split totals, VAT expansion, period state, transactions already in
  the books) and writes nothing. It returns the vouchers, the statement's period and balances, and a
  preparation_hash that binds the actor, the chart, the statement bytes and every
  voucher. Show the vouchers to the user; after approval, seal with
  --approved-preparation-hash. Any change since the preview (statement file,
  assignments, actor, chart) refuses the seal with IMPORT_PREPARATION_CHANGED
  before anything is written. A seal without the flag is not bound to a preview.

EMPTY REPORT
  A statement with no booked entries is not an error: {"assignments":[]}
  previews and seals it with no vouchers, retaining only the file. Parse,
  preview and seal then carry empty_report {account_iban, periods,
  unbooked_count, message}. A wrong account, a wrong period or a truncated
  export looks the same, so confirm with the user that the bank shows no
  transactions for that account and period before sealing it as a quiet month.

Run it:
  aggountant --home ./accounting --json import camt seal --file statement.xml --dry-run < assignments.json
  aggountant --home ./accounting --json import camt seal --file statement.xml \
    --approved-preparation-hash sha256:v1:... < assignments.json"#;

pub(crate) const VERO_FI_CONFIGURE: &str = r#"INPUT (JSON on stdin)
  The fields to change in vero-filing.json. They are merged into the stored
  profile (JSON merge patch): a field left out keeps its stored value, null
  clears it, and members of attestations merge one by one. The output lists
  every changed field under changes, before and after; check them with
  --dry-run first. A whole profile is accepted as well.
  The evidence the verify commands record (attestations.test_verification,
  attestations.portal_verification) is kept: an input that would clear or
  change it is refused (VERO_EVIDENCE_WOULD_BE_DROPPED). --replace writes the
  input as the whole profile instead, dropping every field it leaves out; it
  is also the way to overwrite a stored profile that is not valid
  (VERO_PROFILE_INVALID). Unknown fields are refused.
  schema_version  1. Required when there is no stored profile (or --replace).
  mode            "disabled" or "production"; required when there is no stored
                  profile (or --replace). A production profile is written only
                  when readiness is ready (otherwise VERO_NOT_READY).
  business_id     optional, Business ID 1234567-8 with a valid check digit.
  contact_name    optional, 1..70 characters.
  contact_phone   optional, 5..15 digits with phone punctuation (+ - space ()).
  attestations    optional object, every member optional (false when unset):
                  software_registered, api_agreement_active,
                  production_verification_completed,
                  production_permission_active (booleans),
                  activation_certificate_fingerprint_sha256 (64 hex),
                  test_verification and portal_verification (evidence the
                  verify commands record; leave them out).
  credential_path optional. Platform-local credential bundle path, stored in
                  the platform configuration, not in the profile; absent or
                  null leaves the stored path unchanged.

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
  aggountant --home ./accounting --json vero-fi configure < vero-filing.json
Change one field of a stored profile:
  echo '{"contact_phone":"+358 40 7654321"}' | aggountant --home ./accounting --json vero-fi configure --dry-run"#;

pub(crate) const TULOREKISTERI_CONFIGURE: &str = r#"INPUT (JSON on stdin)
  The Incomes Register channel settings in .aggountant/tulorekisteri.json. They
  are merged into the stored profile (JSON merge patch): a field left out keeps
  its stored value and null clears credentials_file. The output lists every
  changed field under changes, before and after, and the resulting readiness;
  check them with --dry-run first. --replace writes the input as the whole set
  of settings instead. Unknown fields are refused; employer_report is refused
  here and kept as stored, also with --replace (`file-wage employer-report
  configure` writes it). The profile holds no secrets.
  schema_version               1.
  mode                         "disabled" or "production". A production profile
                               is written only when readiness is ready
                               (otherwise TULOREKISTERI_NOT_READY, naming each
                               blocker).
  business_id                  the company's own Business ID, 1234567-8 with a
                               valid check digit; the credential's must match.
  reporting_permission_active  the user's statement that the Incomes Register
                               wage reporting permission is granted (not just
                               applied for). The program cannot check it.
  credentials_file             optional absolute path of a separate credential
                               bundle (same format as the Vero one), outside the
                               home; absent or null uses the Vero credential path.
  The first four are required when there is no stored profile (or --replace).

Example JSON (input-shape demonstration only; fictional company):
```json
{
  "schema_version": 1,
  "mode": "production",
  "business_id": "1234567-1",
  "reporting_permission_active": true,
  "credentials_file": null
}
```
Check first, then write; `tulorekisteri status` shows readiness offline later:
  aggountant --home ./accounting --json tulorekisteri configure --dry-run < tulorekisteri.json
  aggountant --home ./accounting --json tulorekisteri configure < tulorekisteri.json
  aggountant --home ./accounting --json tulorekisteri status"#;

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
                 untagged VAT-account postings remain, which otherwise refuse
                 the start (VAT_ACCOUNTS_UNRECONCILED). reason must be
                 non-empty and vouchers exactly the ones the preview lists; an
                 acknowledgement that does not match, or is given when nothing
                 needs it, is refused (VAT_RECONCILIATION_ACK_MISMATCH).

Example JSON (input-shape demonstration only, not VAT advice):
```json
{"period":"2026-08","sales_vat":{"amount":"33.59","currency":"EUR"},"purchase_vat":{"amount":"45.68","currency":"EUR"},"payee_iban":"FI2112345600000785"}
```
Run it:
  aggountant --home ./accounting --json file-vat start < start.json

"#,
    vero_transport_help!()
);

pub(crate) const FILE_VAT_REPLACE: &str = concat!(
    r#"INPUT (JSON on stdin)
  The `file-vat start` input (see `file-vat start --help`) for a period whose
  return is already filed, plus the reason. Take the figures from `file-vat
  preview` of that period: they include later vouchers sealed with
  `corrects_vat_period` for it. Unknown fields are refused.
  period, sales_vat, purchase_vat, payee_iban
                 required, as for `file-vat start`. payee_iban is recorded
                 only; a replacement Run pays nothing.
  replacement_reasons
                 required, one or more of calculation-error, legal-practice,
                 tax-audit-guidance, law-interpretation (FileVATReturn/v2
                 ReplacementReason CLC, LGL, TXA, LAW).
  business_id, contact_name, contact_phone, vat_reconciliation_acknowledgement
                 optional, as for `file-vat start`.
  Refused: a period with no filed return (VAT_PERIOD_NOT_FILED); a return the
  Tax Administration has not processed yet (VERO_VAT_RETURN_NOT_PROCESSED);
  figures the Tax Administration already shows
  (VAT_REPLACEMENT_CHANGES_NOTHING); an earlier filing whose outcome is open
  (VAT_FILING_UNRESOLVED).

Example JSON (input-shape demonstration only, not VAT advice):
```json
{"period":"2026-10","sales_vat":{"amount":"504.00","currency":"EUR"},"purchase_vat":{"amount":"45.68","currency":"EUR"},"payee_iban":"FI2112345600000785","replacement_reasons":["calculation-error"]}
```
Run it:
  aggountant --home ./accounting --json file-vat replace < replace.json

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
                    The report is due on the 5th of the following month,
                    or the next business day when the 5th is not one. One
                    report may combine a month's payments to one earner; it
                    then carries the latest payment date.
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
                      {"income_type":"331","amount":"...","allowance_codes":[N,...]}
                          daily allowance: one amount and every per diem
                          type it contains, distinct, 1 meal, 2 partial,
                          3 full, 4 international (a month with partial and
                          full per diems is one line with [2,3]). The output
                          of `claim compute` has this shape. A single type
                          may also be given as "allowance_code":N, and a
                          one-type line is printed that way.

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
    {"income_type": "331", "amount": "79.00", "allowance_codes": [2, 3]}
  ]
}
```
Preview, then start with the same timestamp:
  aggountant --home ./accounting --json file-wage preview --timestamp 2026-10-25T12:00:00Z < report.json
  aggountant --home ./accounting --json file-wage start --timestamp 2026-10-25T12:00:00Z < report.json
A replacement of the accepted report (`file-wage correction ... --action
replace`) reads the same shape: the whole report, corrected, with the accepted
report's business_id, source_reference, employee_id, payment_date,
period_start and period_end. `--action cancel` reads nothing:
  aggountant --home ./accounting --json file-wage correction preview wr-DIGEST --action replace --timestamp 2026-11-02T08:00:00Z < corrected.json
  aggountant --home ./accounting --json file-wage correction submit wr-DIGEST --action replace --timestamp 2026-11-02T08:00:00Z --record-hash HASH < corrected.json"#;

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

pub(crate) const CLAIMANT_PLACE_ADD: &str = r#"INPUT (JSON on stdin)
  Every field is required; unknown fields are refused. The person must be in
  the personal-data register (`employee add`). A place is never edited: record
  a corrected place under a new id and correct the entries that name it.
  employee_id   the person the place belongs to.
  place_id      1-64 characters of a-z, 0-9 and '-'; unique for the person.
  kind          company_premises, group_company_premises (a company in the
                same interest group), home, self_arranged_space, customer_site,
                venue (fair, training or meeting venue) or other.
  label         a name the user recognises.
  location      the address distances are measured from.
  municipality  the municipality (kunta); a secondary workplace must be in
                another municipality than the primary regular workplace.
  replaces      optional, the id of a place this one corrects (same place,
                corrected details). The old place can then no longer be named:
                correct the entries that name it to name the new place.
  confirmed_by  who confirmed the place.

Example JSON (input-shape demonstration only):
```json
{"employee_id":"maija","place_id":"home","kind":"home","label":"Koti ja yhtiön toimitila","location":"Esimerkkikatu 1, 33100 Tampere","municipality":"Tampere","confirmed_by":"Maija Meikäläinen"}
```
Run it:
  aggountant --home ./accounting --json claimant place add < place.json"#;

pub(crate) const CLAIMANT_ADD: &str = r#"INPUT (JSON on stdin)
  One dated fact the user confirmed; unknown fields are refused. Record only
  what the user confirms: the program never infers these facts.
  employee_id    required, a person in the personal-data register.
  valid_from     required, the first day the fact holds (YYYY-MM-DD).
  valid_to       optional, the last day it holds; absent means open-ended.
  confirmed_by   required, who confirmed the fact.
  evidence_hash  optional, sha256 of an evidence artifact (`artifact attach`).
  fact           required, one of:
    {"kind":"workplace","role":ROLE,"place_id":"...","basis":BASIS,
     "basis_explanation":"..."}
       ROLE: regular_primary (varsinainen työpaikka), regular_other (another
       regular workplace), secondary (toissijainen työpaikka), tracked_site
       (erityinen työntekemispaikka the register follows) or none (no regular
       workplace, for example an outside board member; no place_id).
       BASIS, with the roles it can justify and the guidance it rests on
       (VH/630/00.01.00/2026):
         works_there_regularly             regular_*   3.1
         company_premises_in_home          regular_*   3.4 (place kind home)
         agreed_home_work_occasional_visits regular_*  3.4, 10.7 (home or
                                                       self-arranged space)
         mobile_work_base                  regular_*   3.5
         remote_work_by_choice             regular_*   10.7 (employer premises)
         employer_premises_weekly          regular_*   5.2.2, 10.7, KHO 2017:178
                                                       (employer premises)
         premises_in_another_municipality  secondary   4.1, 5.2.2
         occasional_visits                 tracked_site 5.2.1
         fixed_term_assignment             tracked_site 5.2.2, 5.3
         board_member_only                 none        10.1
         mobile_work_no_base               none        3.5
         other                             any role; basis_explanation required
    {"kind":"relationship","relationship":R}
       R: employee, managing_director, board_member, unpaid_owner or
       other_fee_recipient (EPL 13 §). contractor is refused: a contractor's
       travel costs belong on their own invoice.
    {"kind":"part_owner","part_owner":true|false}
       Whether the person is a part-owner of the company (the Incomes
       Register's "osakasasemassa maksajaan").
  An entry that contradicts another one in force on overlapping dates is
  refused (CLAIMANT_FACT_CONFLICT): correct the existing entry instead.

Example JSON (input-shape demonstration only):
```json
{"employee_id":"maija","valid_from":"2024-01-01","fact":{"kind":"workplace","role":"regular_primary","place_id":"home","basis":"company_premises_in_home"},"confirmed_by":"Maija Meikäläinen"}
```
Run it:
  aggountant --home ./accounting --json claimant add < fact.json"#;

pub(crate) const CLAIMANT_CORRECT: &str = r#"INPUT (JSON on stdin)
  Unknown fields are refused. The corrected row is never edited: the new row
  supersedes it, so what was known before the correction stays answerable
  (`claimant show --as-known-at`). A row is corrected once; to correct again,
  correct the newest row.
  employee_id    required.
  fact_id        required, the row to correct (`claimant history`).
  reason         required, why the earlier entry was wrong.
  confirmed_by   required.
  evidence_hash  optional.
  replacement    the corrected fact, same shape and kind as in `claimant add`:
                 {"valid_from":"...","valid_to":"...","fact":{...}}
  retract        true to remove the fact without a replacement.
  Give exactly one of replacement and retract.

Example JSON (input-shape demonstration only):
```json
{"employee_id":"maija","fact_id":3,"reason":"The office lease ended at the turn of the year.","confirmed_by":"Maija Meikäläinen","replacement":{"valid_from":"2024-01-01","valid_to":"2024-12-31","fact":{"kind":"workplace","role":"regular_primary","place_id":"office","basis":"works_there_regularly"}}}
```
Run it:
  aggountant --home ./accounting --json claimant correct < correction.json"#;

pub(crate) const CLAIMANT_SNAPSHOT: &str = r#"INPUT (JSON on stdin)
  Unknown fields are refused.
  employee_id            required.
  date                   required, the trip's start date (YYYY-MM-DD): a place
                         is judged by the circumstances when the trip is made.
  start_place_id         optional, the registered place the trip starts from.
  destination_place_ids  optional, the registered places the trip goes to. A
                         destination the register does not name is a special
                         work site (erityinen työntekemispaikka).
  as_known_at            optional RFC 3339 instant; default now.
  Output: the register in force with its facts_digest, each place's role and
  classification, and the trip's workplace as `claim compute` uses it:
  special_work_place, secondary_workplace (travel and lodging only),
  regular_workplace (no tax-free allowance), or null when a secondary
  workplace and a registered special site are both destinations (split the
  trip). Refused: CLAIMANT_RELATIONSHIP_UNKNOWN, REGULAR_WORKPLACE_UNKNOWN,
  TRIP_TO_HOME (the home listed as a destination; list the places the trip
  went to for work), PLACE_REPLACED.

Example JSON (input-shape demonstration only):
```json
{"employee_id":"maija","date":"2026-03-04","start_place_id":"home","destination_place_ids":["customer-turku"]}
```
Run it:
  aggountant --home ./accounting --json claimant snapshot < trip.json"#;

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
            ("import camt seal", parses::<crate::ImportSealInput>),
            (
                "claim compute",
                parses::<aggountant_domain::travel::fi::ClaimInput>,
            ),
            ("vero-fi configure", |json| {
                crate::vero_fi::apply_configure_input(None, json, false)
                    .map(drop)
                    .map_err(|e| e.message)
            }),
            (
                "tulorekisteri configure",
                parses::<aggountant_broker::tulorekisteri::WageProfile>,
            ),
            ("file-vat start", parses::<crate::CliFileVatInput>),
            ("file-vat replace", parses::<crate::CliFileVatInput>),
            ("file-wage preview", parse_wage_report),
            ("file-wage start", parse_wage_report),
            ("file-wage correction preview", parse_wage_report),
            ("file-wage correction submit", parse_wage_report),
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
            (
                "claimant place add",
                parses::<aggountant_domain::workplace::NewPlace>,
            ),
            (
                "claimant add",
                parses::<aggountant_domain::workplace::FactInput>,
            ),
            ("claimant correct", |json| {
                let input: aggountant_domain::workplace::CorrectionInput =
                    serde_json::from_str(json).map_err(|e| e.to_string())?;
                input.into_new_fact().map(|_| ()).map_err(|e| e.to_string())
            }),
            (
                "claimant snapshot",
                parses::<aggountant_domain::workplace::SnapshotRequest>,
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

    #[test]
    fn the_claimant_add_help_lists_every_basis_code() {
        for code in aggountant_domain::workplace::BasisCode::ALL {
            let name = serde_json::to_value(code).unwrap();
            let name = name.as_str().unwrap();
            assert!(
                super::CLAIMANT_ADD.contains(&format!("  {name} ")),
                "basis {name} is missing from the claimant add help"
            );
        }
    }

    #[test]
    fn the_bank_import_help_states_the_date_tolerance_the_ledger_applies() {
        let days = format!("within {} days", aggountant_ledger::DATE_TOLERANCE_DAYS);
        assert_eq!(super::IMPORT_CAMT_SEAL.matches(&days).count(), 2);
    }
}
