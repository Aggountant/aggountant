# Payroll reference data — provenance

The bundled payroll data files follow the same **dated-data-file** pattern as the
chart of accounts (`crates/ledger/data/fi-general-2026.json`): reviewable plaintext,
content-hashed, edited as data (not code) because the values change annually.

## `payroll-rates.json` — statutory rate table (dated editions)

Each edition is valid over `[valid_from, valid_to]` (`valid_to: null` = current).
`resolve_rates(payment_date)` in `crates/domain/src/payroll.rs` selects the edition
whose validity period contains the pay-run's `payment_date`, so a rate change is a
new edition, never a code change.

Percentages are canonical decimal **strings** (never JSON numbers — the ADR-2
string-scalar discipline the rest of the model uses).

- `tyel_employee_pct` — työntekijän työeläkevakuutusmaksu (TyEL), withheld from pay.
- `tyel_employer_pct` — työnantajan TyEL, an employer side-cost.
- `sava_employer_pct` — työnantajan sairausvakuutusmaksu (sava), employer-only.
- `unemployment_employee_pct` / `unemployment_employer_pct` — työttömyysvakuutusmaksu.

**VERIFY (statutory rates).** The 2025/2026 figures here are representative
placeholders for the S4 build-spine slice, NOT a legally verified rate set. Before
any real pay run (the SV real-data track), each edition must be checked against the
official sources for the year:
- TyEL % — Työeläkevakuutusmaksut, Eläketurvakeskus / työeläkeyhtiöt.
- sava % — Työnantajan sairausvakuutusmaksu, Verohallinto.
- Työttömyysvakuutusmaksut — Työllisyysrahasto.

## `tulolajit.json` — income-type (tulolaji) codes

The Tulorekisteri income-type code set, as data. Only a representative subset is
bundled for the slice; the full catalog is large. `payment_date`-independent (the
catalog carries its own `catalog_date` edition marker).

**VERIFY (Tulorekisteri tietuekuvaukset).** The codes and the two report kinds
(palkkatietoilmoitus vs. työnantajan erillisilmoitus, and the 5-day / monthly
deadlines modeled in `payroll.rs`) must be checked against the current Tulorekisteri
record descriptions before a real submission.
