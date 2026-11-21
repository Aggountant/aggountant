# Assessment: migrate-from-chrono

Source review: `history/review-migrate-from-chrono.md` (copied to `issues/migrate-from-chrono/review.md`). HEAD at assessment: `04247ec`.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | XSD conformance test accepts non-xs:dateTime timestamps (`crates/broker/tests/payer_summary_schema.rs:246`)[^1] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX (Rule 5: confirmed, test-local, clean) |
| F2 | Leap-second input folds `:60` to `:59` (`crates/model/src/instant.rs`, `Instant::parse`)[^2] | CONFIRMED | COSMIC_RAY | WORSENS | MINOR | HIGH | DROP (Rule 1b: COSMIC_RAY, the fix would not improve readability) |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 1

[^1]: review line 8 (F1), finder gpt-6-astra; confirmed by verifier claude-fable-5-2 with a chrono-vs-jiff differential harness.
[^2]: review line 21 (dropped list), verifier claude-fable-5-2.

## Evidence

- **F1.** chrono `parse_from_rfc3339` rejected `2026-10-02T09:30Z`, `+03`, `,5` and `[UTC]` forms; `Instant::parse` accepts them, and the XSD declares `Timestamp` as plain `xs:dateTime`, so the helper was the only gate. Generated XML is correct (golden tests pin `HH:MM:SS`); the defect is a weaker test. **Applied** in the landing change: lexical check before `Instant::parse` plus two rejection cases, `payer_summary_schema` green.
- **F2.** Reachable only from a hand-typed `--timestamp …:60Z` under the chrono build; ADR-9 accepts the fold. No issue staged: no observed occurrence and no credible impact.

No follow-up issues were staged.
