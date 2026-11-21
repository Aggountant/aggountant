# Review: replacing an externally filed month needs its stated pension insurances

`/llm-review`, routine rigor. Finder `claude-fable-5-2`, verifier `gpt-6-astra`
(two model calls). Scope: the working-tree diff of this slice (process planner,
CLI flags, tests, user guide, `skills/tulorekisteri/kiinnikurominen.md`).
Risk class: authority filing (a replacement sent to the Incomes Register cannot
be taken back), so the verifier was called on the surviving candidates.

## F1 (should-fix): `--previous-pension-insurance` help promises a refusal that only applies to "No wages payable"

- Location: `crates/cli/src/wage.rs`, help of `--previous-pension-insurance`;
  `crates/process/src/file_employer_report.rs`, the `HealthInsuranceContribution`
  arm that carries the previous insurances forward.
- Claim: the help says a difference from the profile is refused
  (`EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED`). For a EUR 0 contribution
  replacement the stated list becomes the report's list, so the comparison is
  stated against itself and a stated list that differs from the profile is sent
  with only a generic warning.
- Evidence: local reading of the planner and `sent_pension_insurances`; the
  slice's own test sends a stated policy over an empty profile. Verifier:
  survives (carry-forward is intentional; the help is wrong, and the warning
  does not name the profile mismatch).
- Fix: qualify the help; warn specifically when the stated list differs from the
  profile on a contribution replacement.

## F2 (should-fix): the pension-insurance comparison is order-sensitive

- Location: `crates/process/src/file_employer_report.rs`, the
  `EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED` check.
- Claim: ordered slice equality. A stated list typed in another order than the
  profile's (both allow up to 20 entries) is refused with advice to cancel the
  report in the e-service, although the insurances are the same.
- Evidence: neither the CLI nor the profile reader sorts; verifier: survives.
- Fix: compare as a multiset; when the previous list was stated, name both
  lists in the refusal so a typo is not mistaken for a needed cancellation.

## Dropped

None. The finder's "checked, not findings" (match arms, approval binding,
retry after acceptance, clap `requires`, `explain` resolving the codes) agree
with local reading.

## Coverage and residual risk

Read: the full planner, CLI args, domain `sent_pension_insurances`, the
updated and new tests (all passing, `scripts/ci-local.sh` green). Not
verifiable from the repository: whether the register rejects a replacement
whose pension insurances differ from the stored version. The slice does not
rely on it. The claim that no interface returns a report's content rests on the
Incomes Register transcript documentation (600/630/312 are sums only; the
e-service "Submitted reports" view shows the report).
