# Result: transcript-630-real-header

The transcript 630 parser (`crates/process/src/transcript_630.rs`) now
expects the header and earnings report types the Incomes Register e-service
actually writes:

- columns 26–29 are `… perusteena olevat perusteettomaksi eduksi ilmoitetut
  tulot` (`HEADER_FI`);
- earnings rows are `Palkkatietoilmoitukset: …` in the plural
  (`TYPES_EARNINGS`).

Everything else stays as strict as before: any other header, row type or
period is refused and nothing is stored.

## The documented §4.6.2 wording is not accepted

Decision: the document's header (`… perusteettomat edut`) and singular
earnings type are refused like any other unknown layout.

Reason: no file has been seen to use them. Both the e-service API and the
browser download produce the wording above, so accepting the document's form
would add a second header the parser can never test against reality, and the
module's rule is to accept only what a real file verifies. Being wrong is
cheap and loud: such a file is refused with `TRANSCRIPT_UNKNOWN_HEADER`,
shows the line found, stores nothing, and the fix is one constant. Test:
`the_documents_wording_is_not_the_e_service_header`.

Not verified: the two substitute-payer types (`Sijaismaksajan maksamat` and
`Sijaismaksajana maksetut suoritukset`) are pluralised by analogy; the real
files had only `Maksajan omat suoritukset`. A file that spells them
differently is refused (`TRANSCRIPT_INVALID_ROW`), not misread.

## Empty contribution cells

An empty employer's health insurance contribution or deduction cell already
read as zero; the count must still be a whole number. The fixture
`transcript-630-2025.csv` now has a month (July) with one report and both
cells empty, as in the real files, checked by `real_e_service_file_parses`.

## Fixtures and wording

- Both fixtures in `crates/process/tests/fixtures/employer_report/` carry the
  real 33-column header and the plural type, with invented amounts (UTF-8 with
  BOM and CRLF, like the browser download).
- `TRANSCRIPT_UNKNOWN_HEADER` messages, the guide (fi/en task page and error
  codes) and `skills/vero-fi/SKILL.md` now speak of the e-service's columns
  instead of "the documented columns".
- The issue text was stripped of the real company's name, host and file
  location (it was already on `main`; git history still has them).

## Left to the conductor

The last acceptance criterion, `import-transcript --dry-run` on the company's
four real files with every month 2/2020–10/2026 `recorded`, needs the private
files and is run by the conductor after landing.

## Gates and review

`scripts/ci-local.sh` green. `/llm-review` (`review.md`, `gpt-6-astra`
finder, no verifier needed): no findings. `/assess-findings`
(`assessment.md`): nothing to triage.

## Simplicity

The change edits the existing constants in place; no alternate header table,
no configuration and no new error code. The one rule (header equals
`HEADER_FI`) stays in one place.
