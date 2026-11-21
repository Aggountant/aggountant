# Review: vat-replacement-return

- Skill: `/llm-review` on commit `ce6c1e9` (the slice before the review fixes).
- Finder: `claude-fable-5-2`. Verifier: `gpt-6-astra` (one batched refutation call).
- Model calls: 2. Rigor: routine; verifier used because candidates survived
  local checking and the surface is high-risk (irreversible authority filing,
  fiscal store, VAT figures).
- Scope: `git diff HEAD~1 HEAD` of the replacement VAT return slice (model,
  domain, ledger, broker, process `file_vat*`, cli, mock, tests, skills, user
  guide).
- Evidence: every finding below was checked by reading the cited code; the
  verifier's verdict is given per finding. No verdict rests on agreement.

## Finding 1 (must-fix): coverage of a correction is decided by the filing event's timestamp

- Location: `crates/ledger/src/lib.rs:1078` (`unfiled_vat_corrections`);
  consumers `crates/process/src/file_vat.rs:309`,
  `crates/process/src/file_vat/status.rs:160`.
- Claim: a correction voucher counts as "not yet filed" only when it was sealed
  after the latest filing event was *recorded*. The request bytes are fixed at
  dispatch, but `file-vat reconcile` can record the filing days later. A second
  correction sealed in between is older than the event, so it is reported as
  covered although the return in force does not contain it; `in_vat_period`
  also keeps it out of its posting period's return. Its VAT is in no return and
  nothing says so.
- Evidence: local read of `unfiled_vat_corrections`, the seal gate
  (`sqlite_fiscal.rs`, requires only Sealed + filed), `record_filing` called
  from `reconcile` (`recovery.rs`). Verifier: SURVIVES.
- Impact: silently unreported VAT after an unknown-outcome replacement that is
  adopted later. Rare sequence, but silent.
- Fix: decide coverage by when the request was fixed: the first
  `EffectExecuting(submit-filing)` of the Run that holds the in-force
  `sent_hash`; keep the recorded time only for a filing no Run holds (manual).

## Finding 2 (should-fix): `complete-manual --replacement` appends a filing on every call

- Location: `crates/process/src/file_vat.rs:446`, `crates/ledger/src/lib.rs:2230`.
- Claim: the manual replacement reads the current filing count and mints its
  authority against it, so the CAS always passes, and the replay branch needs a
  `sent_hash`, which manual evidence does not have. Repeating the command with
  the same evidence records a third filing for two returns.
- Evidence: local read; a repeated first manual filing is refused, this path is
  not. Verifier: SURVIVES.
- Impact: the period's filing history shows a return that was never made, and a
  previewed replacement Run is refused because the count moved.
- Fix: when the latest filing already carries the same channel and evidence
  reference, replay it.

## Finding 3 (should-fix): an interrupted Run is judged by the latest filing, not the one in its slot

- Location: `crates/process/src/file_vat/recovery.rs:257-270`, `filing_landed`
  in `crates/process/src/file_vat.rs`.
- Claim: `reconcile` of an interrupted Run decides "filed manually" from the
  latest filing's evidence. Sequence: first-return Run R1 interrupted; the
  period is filed in OmaVero and recorded with `complete-manual`; later a
  replacement lands through the API. `file-vat reconcile --run-id R1` then finds
  a latest filing that is not manual, journals "fiscal wins" and opens
  `ApprovePayment` on R1 for a return it never filed. Before this commit the
  same Run ended `Superseded`.
- Evidence: local read of the route selection and `manual_vat_filing_evidence`
  (now the latest filing). Verifier: SURVIVES, noting it opens a payment
  approval, not a payment.
- Impact: a Run claims a filing it did not make and offers a payment approval
  for tax already handled.
- Fix: look at the filing that took the Run's slot
  (`vat_filings[replaces_filings]`): the Run's own `sent_hash` means the fiscal
  record landed; anything else supersedes the Run.

## Finding 4 (should-fix, uncertain): the replacement preflight does not check that the processed answer is the return in force

- Location: `crates/process/src/file_vat.rs:1767-1806` (`read_replaced_return`).
- Claim: the preflight accepts any `Processed` answer as the return being
  replaced. If the authority can show an earlier return as `Processed` while a
  receipted newer one waits, the Run stores the wrong baseline: its "not shown"
  answer never recurs and the preview's changes are computed against the wrong
  return.
- Evidence: the check is only `Status == "Processed"`. Whether the API gives
  that combination is not stated in the portal text ("If a return is being
  processed the values of the last accepted return are shown"). Verifier:
  UNCERTAIN (external fact); notes that the manual route still resolves it.
- Fix: when the return in force has an API receipt, refuse a preflight answer
  submitted before the receipt's date, as `file-vat status` already reads it.

## Finding 5 (should-fix, uncertain): status can call a same-day pending replacement `processed_values_differ`

- Location: `crates/process/src/file_vat/status.rs:627-650`.
- Claim: a pending replacement is told from the earlier return by
  `SubmittedDate < receipt date`. When the replaced return was submitted on the
  day the replacement was receipted, the unchanged earlier answer is compared
  with the replacement's values and reported as a processed return that
  differs.
- Evidence: local read; the misclassification is definite for that answer.
  Verifier: UNCERTAIN on whether the API answers so; notes the message still
  ends "do not file this period again".
- Fix: an answer identical to the holder Run's stored prior return is the
  earlier return: report `pending_authority_processing`.

## Unresolved uncertainty

- Which `Status` GetFiledVATReturn/v2 gives the previous return while a newer
  one is waiting (decides how often 4 and 5 can occur). Not in the portal text.
- The closing rule for a replacement (the unchanged earlier return twice, at
  least a bank day apart) is also what the authority shows while an accepted
  replacement waits for processing. A first return has the authority's own
  backstop against a second "first" return; a re-sent replacement with the same
  values has none, and is harmless in effect (the same return) unless figures
  changed in between, which the new preview shows. As designed; recorded here
  for the person judging the slice.
- A replacement recorded with `complete-manual --replacement` has no request
  to compare, so corrections sealed before it was recorded count as covered by
  it. The operator's evidence is the only record of what was filed.

## Dropped candidates

0 dropped. The finder's "checked and found sound" list (fiscal chain CAS and
verify fold, hashing of `corrects_vat_period`, VAT selection through
`vat_period_vouchers`, older Run state files, the dispatch guards) matches what
the tests exercise.

## Coverage

Read: the whole diff; `recovery.rs`, `status.rs`, `file_vat.rs`, the ledger
store transition and seal gate. Tested: workspace tests and `ci-local` pass on
the reviewed commit. Not covered: the Finnish guide text was not read by the
finder; no call was made to any Vero environment (the scripted seam and the
mock only).
