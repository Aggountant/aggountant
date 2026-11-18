# Assessment of the employer-separate-report review

| # | Finding | Classification | Outcome | Notes |
|---|---|---|---|---|
| F1 | Stale or versionless e-service evidence settled an in-doubt replacement | CONFIRMED | FIX | Data integrity: the books would name an unarrived replacement as the register's current report. Fixed with the verifier's `replaces_version == None` extension; two tests |
| F2 | 101-with-policy → 102 replacement dropped `PensionInsurances` | CONFIRMED | FIX | Register processing rule §2.9 verified in the official PDF. Fixed by carrying the previous version's insurances forward, which matches guidance §4.2 (correct with a replacement) and §5 (cancel-and-new only for a changed policy). Worst case if the register disagrees: a definitive rejection |
| F3 | Deadline ignores weekend/holiday relief; `overdue` may be early | CONFIRMED (behaviour), not a defect | DROP | Documented conservative choice; never makes a report late. In `result.md` decisions |
| V1 | Replacement of an e-service-recorded report skips the pension comparison | CONFIRMED | DROP | Previous insurances unknown; outcome at worst a definitive rejection |
| V2 | E-service cancellation cannot be recorded | CONFIRMED (pre-existing) | SPIN_OFF | Filed unlaned as `employer-report-record-cancellation` |

No confirmed data-integrity finding remains open.

## Simplicity

- Reused: the `file-wage` own-organisation channel (same broker, credential,
  profile file, signing, outbox, `StatusService` query), the process journal
  and its event types, and the existing `run_dir`/lease machinery. No second
  channel, no new journal events.
- Repetition: `sent_pension_insurances()` is the single rule for what pension
  data leaves; both the XML builder and the replacement guards use it. The
  employer-report receipt is a small `note` codec next to the run model instead
  of a second receipt store.
- Left out on purpose: computing the health insurance contribution from wages
  (a month with wages is refused), weekend/holiday deadline relief, recording
  cancellations, and the 2027 schema changes (noted in the vendored README).
