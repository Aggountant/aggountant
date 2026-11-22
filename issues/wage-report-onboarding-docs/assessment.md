# Assessment of the wage-report-onboarding-docs review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Documented pipe regenerated the approval | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX — applied |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: an agent that edits the report file after showing the preview (to fold
  in a second correction, say) and then follows the documented pipe would send
  content the user never saw. That is silently wrong filing, so it is fixed
  here: the hash is captured from the reviewed preview, never recomputed at
  approval time.

No follow-up issue was filed.

## Simplicity

- Reuses: the existing `--record-hash` value path; the new form only resolves
  to the same string before the unchanged engine call.
- One definition: the three approvals share one flattened `RecordHash`
  argument group and one reader, so the input rule, the error text and the
  "exactly one of the two" rule are stated once.
- Left out: reading the hash from the stored run ("stored preview" in the
  issue). The run's gate already holds the hash; approving with it would make
  the approval name nothing the user saw, which removes the check rather than
  hiding the value. Also left out: a separate stdin-only flag; `-` covers it.
