# Assessment of the camt-event-identity review

Assessed against the review fixes on top of `62b6acb`.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Projection hash depended on the release | CONFIRMED | REGULAR | IMPROVES | MINOR | HIGH | FIX — applied |
| F2 | Booked transactions had to pass today's content checks | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX — applied |
| F3 | Pending-and-booked reference conflicted with itself | CONFIRMED | RARE | NEUTRAL | NONE | MED | FIX — applied |
| F4 | Empty request hash in unbooked-conflict rejection | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | FIX — applied |
| F5 | `backfilled_count` non-zero where nothing was written | CONFIRMED | REGULAR | NEUTRAL | NONE | HIGH | FIX (doc) — applied |
| F6 | Fingerprint covers display fields; hard stops have no exit | CONFIRMED | RARE | NEUTRAL | MODERATE | LOW | DISCUSS |
| F7 | Conflict rejections only on a race | CONFIRMED | — | NEUTRAL | NONE | HIGH | DROP (1d: pre-existing behaviour of every import refusal; documented) |
| F8 | Unreadable legacy statements re-parsed per import | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | DROP (1b: negligible cost) |
| F9 | Backfill contradicts the issue's out-of-scope note | INCORRECT | — | — | — | HIGH | DROP (superseded by the conductor decision) |

FIX: 5   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 1   DROP: 3

- F1 is the one that mattered: without it, the first release that touches the
  VAT rates file would have made every overlapping re-import of a booked VAT
  transaction stop, with advice that cannot help. Fixed before any home stores
  a projection hash, so no stored data needs changing.
- F2 and F3 are fixed with tests (F3 in the parser, plus the check order).
- F6 is a product question: whether a booked transaction may be re-imported
  without restating its assignment, and whether display-only bank fields should
  stop an import. Both change the issue's design and go to the conductor.

No confirmed data-integrity finding remains open.
