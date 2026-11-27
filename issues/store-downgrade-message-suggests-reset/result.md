# Result: store-downgrade-message-suggests-reset

Every store refusal now says only what is safe. All store refusals (fiscal and
pii alike) go through `crates/ledger/src/store/migrations.rs`, so the wording
changed in one place:

- **Downgrade** (`user_version` newer than the binary knows): "… the database
  was written by a newer aggountant, so it was not opened and nothing was
  changed. Do not modify, delete or reset this home: it may hold the
  company's live books. Open it with the aggountant version that wrote it, or
  a newer one."
- **Pre-framework** (`user_version` 0 with tables): "… so it was not opened
  and nothing was changed. Do not modify, delete or reset this home: … Open
  it with the aggountant version that wrote it."
- **Negative `user_version`**: adds "nothing was changed"; it never suggested
  a reset.

The shared sentence is one constant (`KEEP_THE_HOME`). Error codes are
unchanged (`STORE_ERROR`). The module docs and test comments that called the
data "resettable" were rewritten.

A reset is legitimate only for a pre-framework home that holds no real books
(for example a practice home). The guide's troubleshooting page (fi and en)
has a new section, "Ohjelma ei avaa kotihakemiston tietokantaa" / "aggountant
does not open the home's database", which says to install the version that
wrote the home (or newer), states that one condition for deleting and
re-creating a home, and forbids editing, deleting or resetting a home with
real books.

Grep of the workspace (`crates`, `skills`, `docs/user`, root docs) for
"resettable", "reset this home", "start over", "re-initialise" and similar
found no other reset advice in errors, hints, skills or the guide. The
remaining "resettable" hits in `docs/site/adr-3-fiscal-store.md` are about
`PRAGMA query_only`, which is unrelated.

## Tests

- `migrations::tests::assert_refusal_gives_no_reset_advice` fails if a refusal,
  apart from the prohibition sentence, mentions reset, resettable, delete,
  remove, recreate, re-run init or start over.
- Applied to the downgrade, pre-framework and negative-version refusals in
  `migrations.rs`, and through the real stores: new
  `a_newer_fiscal_store_is_refused_without_reset_advice` (also checks the
  version stays untouched), `a_newer_pii_store_is_refused_without_reset_advice`,
  and the existing pre-framework tests of both stores.

No store migration; schema versions unchanged.

## Review

Focused self-review, proportionate to a change that touches only error text,
comments, tests and the guide: no store behaviour, schema or bookkeeping
change. Checked that format strings render the constant (tests assert the full
sentence), that the fiscal downgrade test leaves `user_version` as it found
it, and that the guide section's steps match the program (`init` does not
touch an existing home, so the deletion comes first and only under the
stated condition). `scripts/ci-local.sh` passed, including the user guide
gate and the site build.
