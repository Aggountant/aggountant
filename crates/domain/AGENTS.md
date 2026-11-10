# crates/domain — rules the code does not show

Read `crates/AGENTS.md` first: the Domain is pure, deterministic and never
guesses. The rules below concern VAT, where a wrong answer is frozen into a
sealed voucher and can only be undone by a correction voucher.

- **VAT legs are expanded before the seal and live in it.** Expansion turns
  gross amount, treatment, net account, document date and the pinned policy set
  into the full posting set; the sealed voucher carries every VAT leg plus the
  treatment provenance (treatment, source amount, accounts, resolved rate,
  `policy_set`, rate-table hash, generated-leg markers). A later rate change
  therefore cannot alter an old voucher. A verifier may re-run expansion and
  compare, but the sealed bytes are the authority.
- **Treatment is never inferred from accounts.** Generated-leg markers say which
  postings are VAT legs, chart roles say an account can carry such a balance,
  and the sealed treatment decides the category. The same input/output account
  pair may serve several treatments when the active chart selects it; account
  numbers are not part of the Domain contract and come from the exact active
  chart snapshot. A missing role refuses loudly; never fall back to default
  accounts or another chart version.
- **Return fields come from sealed treatment, never from account identity.** The
  filing projection sums hash-bound generated legs and selects fields by
  treatment. A sealed treatment the field map does not cover is refused rather
  than filed as an aggregate. The `rate_class → 301/302/303` rule has one owner
  in the filing computation; do not repeat it in chart metadata or infer it
  from a percentage. The projection lists the contributing vouchers per field so
  a shared account stays auditable.
- **Categories are closed and unmodelled cases refuse.** `NonEuServices` means
  services only; goods imported from outside the EU need customs-based
  treatment that does not exist yet and must not be classified as non-EU
  services. EU goods, special-rule EU services, domestic reverse charge and
  similar cases need their own explicit category or are refused.
- **An uncertain treatment is not sealed as a guess.** It stays unsealed (intake),
  or, for a known bank event that must be recorded by a deadline, it is sealed
  with a justified provisional treatment (a suspense account or non-deductible
  VAT) and corrected later. "VAT unclear" is never a reason to omit a known
  transaction.
- Every rate change ships policy test vectors, including one that crosses the
  change date, because expansion is a pure function and the easiest place in
  the system to pin with golden vectors.
