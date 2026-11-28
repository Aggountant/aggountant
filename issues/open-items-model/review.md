# Review: ADR-12 (open items), `docs/site/adr-12-open-items.md`

- **Skill:** `/llm-review`, rigor routine (a proposed decision document; the
  migration it shapes is reviewed again in its implementing slice).
- **Finder:** `gpt-6-astra` (one call). **Verifier:** `claude-fable-5-2` (one
  call, both surviving candidates batched). Two model calls in total.
- **Scope:** the ADR text, checked against `crates/AGENTS.md`,
  `crates/ledger/AGENTS.md`, `crates/ledger/src/store/sqlite_fiscal.rs`
  (migrations 1–9), `crates/ledger/src/bank_event.rs`, ADR-11 and the issues
  `expense-claim-approve-seal`, `expense-claim-payment-matching`,
  `expense-claim-status`, `open-items-model`; and against the cited primary
  sources: KPL 1336/1997 2:2.3 and 2:3 (Finlex consolidated text, read
  2026-11-28), KILA's general guidance of 20.4.2021, 1.4.1 and 5.7 (PDF), and
  the Finanssiala ISO 20022 Account Statement Guide 2020 (BOOK/INFO/PDNG
  statuses).
- **Risk class:** design document for a fiscal-store migration and a
  double-payment control.

## Confirmed findings

### F1 (must-fix): the partial-payment recommendation left the paid part offered again

- **Location:** Open questions, question 5.
- **Claim:** "Book the paid part as a receivable from the claimant and offset
  it when the rest is paid" leaves the payment unlinked to the claim. The
  claim's outstanding amount stays at the full approved amount, so the
  pay-now list (section 4) offers the full amount although part is paid. The
  question 7 guard does not fire, because the posting is on the receivable
  account, not the claims payable.
- **Evidence:** local: outstanding is computed only from origin and
  settlement rows (section 1, section 2); "Approved, unpaid" is "version
  link, no settlement" in `expense-claim-status`; question 6's
  `claimant_receivable` item covered only a second payment. Verifier:
  survives; harm bounded to the paid part, not a full duplicate.
- **Fix applied:** the receivable is recorded as a `claimant_receivable` item
  naming the claim, and while it is open the claim's status and the pay-now
  list show the outstanding amount less the receivable. The reason for not
  linking partially (two Incomes Register payment dates, no source on how the
  lines split) is now stated. The slice table gives this to
  `expense-claim-payment-matching`.

### F2 (should-fix, verifier: precision only): KILA 5.7 quoted without its condition

- **Location:** Context, "A payment proposal is not a voucher."
- **Claim:** KILA 5.7 excludes payment proposals and lists "elleivät ne
  sisällä KPL:n tai AVL:n perusteella määriteltyjä tietoja, jotka eivät
  ilmene tositteista, niiden liitteistä tai liiketapahtumia koskevasta
  kirjeenvaihdosta"; the ADR stated the exclusion as unconditional.
- **Evidence:** the KILA PDF, section 5.7 (read locally). Verifier: the
  omission is real; no decision in the ADR depends on the unqualified reading.
- **Fix applied:** the condition is added in the ADR and the bookkeeper
  companion.

## Dropped candidates

None. The finder raised two candidates; both survived local checking.

## Coverage and residual risk

Checked: every code claim in the ADR (fiscal store at version 9,
`bank_event` and `idempotency` outside the voucher hash, `adjustment_attribution`
hash-chained, `DATE_TOLERANCE_DAYS = 7`, batch conflicts recorded in the
rejection log, the claimant register's supersede/retract pattern) and every
legal citation. Not checked: anything that depends on what banks' statements
or AI connectors provide; the ADR marks those as assumptions.
