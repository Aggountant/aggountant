---
created: 2026-11-13
updated: 2026-11-16
type: bug
status: in-progress
priority: normal
provenance: ai-review
source_ref: taskfleet:01m7a1vmb1vv3rhg8nx3dx98sq/review-finding:verifier-unlisted-settlement-gaps
review_source: ai-review
originating_run: 01m7a1vmb1vv3rhg8nx3dx98sq
originating_run_kind: spinoff
labels:
- ai-review-model:claude-fable-5-2
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_confidence: high
review_severity: should-fix
review_target: f7ace2b..HEAD (vat-settlement-with-interest-flagged)
lane: vat-calculation
lane_seq: 0
collision: [crates/domain/src/vat/reconcile.rs, crates/ledger, crates/process/src/vat_reconciliation.rs]
preparation: ready
---

# Untagged VAT reductions pass as VAT payments and drop out of the return

## What goes wrong

Before a VAT return is filed, aggountant compares the VAT accounts with the
return and stops when VAT booked without a VAT code would be left out. VAT
payments to the tax authority and refunds from it are not part of a return, so
the comparison sets them aside. It recognises them only by what the voucher
looks like, and some ordinary corrections look the same: a refunded customer
advance, a returned fixed-asset purchase, a private-use correction of input
VAT. If such a correction was booked without a VAT code, aggountant takes it
for a VAT payment, says the accounts agree and files a return that still
contains the VAT the correction removed. Nobody is told. The company has then
filed the wrong VAT, and finding out later needs a replacement return.

On 2026-11-16 the product owner decided how to fix it: the chart of accounts
marks the accounts VAT is paid and refunded through (bank accounts and the tax
account), and only a voucher whose money side goes through such an account
counts as a VAT payment or refund.

## How detection works now

`classify_vat_account_postings` in `crates/domain/src/vat/reconcile.rs`
classifies every posting on a VAT-role account. An untagged posting on the side
that reduces the VAT balance (output VAT debited, input VAT credited) is a
`Settlement` when `settles_vat` passes: every account is known to the chart,
one currency, and the voucher's income and expense postings sum to less than
the reduced VAT and are only expense debits beside output VAT or only income
credits beside input VAT (interest and penalties). Nothing looks at the other
balance-sheet legs. A `Settlement` is subtracted from the comparison and listed
in `settlement_postings` for audit, but raises no attention item. The same
classifier drives `voucher validate`'s `VAT_POSTING_UNTAGGED` warning (`Ledger::untagged_vat_warnings`
in `crates/ledger/src/lib.rs`) and the period check in
`crates/process/src/vat_reconciliation.rs`, so both follow any change here.

Shapes that pass today although they belong in the return (Esimerkki Oy, all
untagged):

- advance-payment refund: 2620 D 100,00 / 2939 D 25,50 / 1910 C 125,50
- fixed-asset purchase return: 2700 D 1 255,00 / 1150 C 1 000,00 / 1763 C 255,00
- private-use input-VAT correction: 2000 D 25,50 / 1763 C 25,50
- an asset-sale credit note booked against the asset or a receivable

None of these has an income or expense line, so `settles_vat` sees "no
result lines" and accepts them. Note that the first one does have a bank leg:
"a bank leg is present" is not enough. The leg that disqualifies it is 2620.

## The rule to build

A reduction-side untagged VAT posting is a settlement when every other posting
in its voucher is one of:

- a posting on a VAT-role account (this keeps the clearing 2939 D / 1763 C a
  settlement),
- a posting on a settlement-role account of the voucher's own chart snapshot,
- an interest or penalty line that passes today's result-line test (kind,
  side, and smaller in sum than the reduced VAT).

Any other balance-sheet posting (advances received, fixed assets, payables,
receivables, equity) makes the reduction `Untagged`. Keep the existing tests'
outcomes, including the pinned case where a net payment clearing 1763 with
interest flags its 1763 leg; the new rule should only flag more, never fewer.

The role is detection evidence only. It does not enter `derive_vat_figures` or
any filing figure, and it says nothing about VAT treatment, so the rule that
treatment is never inferred from account numbers is not touched.

## Where the role lives

The role belongs to an exact `chart_version`, like the VAT roles, because
account numbers mean nothing outside their chart. In the bundled chart
(`crates/ledger/data/fi-general-2026.json`, `fi-general-2026@dd5d3f33…`) the
accounts are **1910 Pankkitili** and **1920 Pankkitili 2**. Leave out 1900
Käteisvarat: cash sales and cash refunds pass through it, and a role there
would let an untagged cash refund pose as a VAT payment. The bundled chart has
no tax-account (verotili) account, and 1730 Muut lyhytaikaiset saamiset is too
general to carry the role. A company that books its tax account on a dedicated
account has its own chart and gives that account the role in its own role
data. The format should allow an asset or liability account. It should refuse a
VAT-role account, an income, expense or equity account, or an account missing
from the chart snapshot, the way `validate_chart_vat_roles` in
`crates/ledger/src/store/git_masterdata.rs` checks VAT roles today.

Put the role in a **new masterdata file** (for example
`masterdata/reference/settlement-account-roles.json`, bundled from
`crates/domain/data/` beside `vat-account-roles.json`), not as a new field in
`vat-account-roles.json`. The reason is how existing homes are upgraded.
`home upgrade` (`GitMasterDataRefStore::upgrade`) only creates files that are
missing. A file that exists with other bytes is reported under `skipped` as
`differs` and left alone, because it may be company-approved masterdata
(`crates/ledger/AGENTS.md`, "Existing homes take in new masterdata only by
addition"). Every home created by `init` already has `vat-account-roles.json`.
If the role went into that file, the bundled bytes would change, every existing
home would report its role file as `differs` from then on, and no existing home
could get the role short of a hand edit, which the user guide tells people not
to make. A new file is missing in every existing home, so `home upgrade` adds it
as a new `UpgradeResource`. Add it on the same condition as the VAT roles: only
when the bundled file has roles for the home's active chart, and `unavailable`
otherwise. `init` writes it for new homes.

## Homes without the role

The file will be missing in every existing home until `home upgrade` runs, and
a home with its own chart may never get it from the bundle. `open` should still
open such a home: the role only affects detection, and refusing every command,
sealing included, until an upgrade is out of proportion.

The classifier then fails closed. A VAT payment whose bank leg has no
settlement role is `Untagged`, the period is `unreconciled`, and `file-vat
start` refuses until the operator either upgrades the home or acknowledges the
vouchers with the existing `vat_reconciliation_acknowledgement`. Treating it as
a settlement would bring back the silent omission. To keep that from reading as
"your books are wrong", the preview should say what is missing: the voucher's
chart has no settlement roles, and `aggountant home upgrade` adds them for the
bundled chart. A dedicated attention code alongside `VAT_ACCOUNTS_UNRECONCILED`
fits. Likewise, `voucher validate` on such a draft should not advise putting a
VAT code on the base line, which is wrong for a payment.

## Older vouchers and filed periods

`crates/ledger/AGENTS.md` says chart role metadata is frozen once vouchers are
sealed against a `chart_version`, and that a role change is a new chart
edition. That rule protects filed figures and treatment from being
reinterpreted. The settlement role changes neither. Adding it to
`fi-general-2026@dd5d3f33…` makes the reconciliation of earlier periods
stricter: `file-vat preview` on an already filed period can now show it
`unreconciled` because of an old untagged advance refund. That is the intended
discovery, because the filed return was wrong. State this scope in that
`AGENTS.md` rule so the next reader does not take the new file as a breach of
it.

## Not closed by this

A customer refund or credit loss booked untagged on an **expense** account (or
a supplier refund on an **income** account) in the same voucher as a larger VAT
payment still counts as a settlement. Example: 2939 D 500,00 / 7780 D 100,00 /
2939 D 25,50 / 1910 C 625,50. The 7780 line has the same shape as late-payment
interest, and the payment does go through the bank, so a bank role cannot tell
the two apart. It takes a role for interest and penalty accounts or a marker on
the voucher, which is outside this decision. Pin the current behaviour with a
test and keep it in the known-gaps list in the guide or the result, so that a
later change to it is deliberate.

## Acceptance criteria

- The three example vouchers above are `Untagged`, and the period is
  `unreconciled`, in a home whose chart gives 1910 and 1920 the settlement
  role. `voucher validate` warns `VAT_POSTING_UNTAGGED` on each draft.
- A VAT payment (2939 D / 1910 C), a payment with interest and a penalty, a
  refund received with interest income, and a 2939 / 1763 clearing stay
  settlements in that home. Every existing test in `tests_reconcile.rs` keeps
  its outcome once the test role table gives 1910 the role.
- The same VAT payment in a chart snapshot without settlement roles is
  `Untagged`. The preview names the missing roles and `home upgrade`, and an
  acknowledgement naming the voucher lets `file-vat start` continue.
- `init` writes the settlement role file. On a home that lacks it, `home
  upgrade --dry-run` lists it as a create and `--apply` creates it without
  changing `vat-account-roles.json`, the active chart or any other existing
  file. A second run does nothing, and a home with its own chart gets
  `unavailable`. Covered in `crates/ledger/tests/masterdata_upgrade.rs` and
  `crates/cli/tests/home_upgrade_cli.rs`.
- A role file naming a VAT-role account, an income, expense or equity account,
  or an account missing from the chart snapshot makes the masterdata refuse to
  open, with the account named.
- The gap that stays open (expense-booked refund beside a payment) is pinned by
  a test and listed as known.
- User guide fi + en: `tasks/file-vat-omavero.md` says a payment or refund is
  recognised by its bank or tax-account leg, `tasks/upgrade.md` mentions the
  new file, and `reference/error-codes.md` lists any new attention code.
  `crates/ledger/AGENTS.md` states the scope of the frozen-roles rule.

Found by the focused review of `vat-settlement-with-interest-flagged`. The
earlier known-gaps lists are in `issues/vat-untagged-posting-omitted/result.md`
(decision 1) and `issues/vat-settlement-with-interest-flagged/result.md`
(decision 1).
