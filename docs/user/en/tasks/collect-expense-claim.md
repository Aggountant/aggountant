# Collect an expense claim

In aggountant an expense claim (kululasku) is collected as a draft, one trip
and one receipt at a time, over days or weeks. A draft can hold a single trip
or a whole month of trips and receipts. After each addition aggountant tells
you what the draft still lacks and computes the trips' tax-free allowances.
The draft records nothing in the books.

## Before you start

- The claimant's relationship to the company and regular workplace are
  recorded: [Record your regular workplace](record-regular-workplace.md).
  Without them no tax-free allowance and no travel cost is computed for a
  trip.
- You know what the law requires a travel claim to show: the purpose of each
  trip, where it went, when it started and ended, how you travelled, and for
  drives with your own car the route and the kilometres.
- You have the receipts as files or photos.

## How to do it

1. **Tell the agent about a trip or a receipt.**

   > Start Maija Meikäläinen's March expense claim. On 10 March she visited
   > a customer in Tampere in her own car, leaving home at 7:30 and coming
   > back at 19:15. The customer offered lunch. Parking cost €12.40; the
   > receipt is attached.

2. **The agent** adds the trip and the receipt to the draft. The receipt goes
   into the company home's attachment store at once, and the draft keeps its
   identifier. If the agent proposes drives from a calendar or e-mail, a drive
   enters the draft only when you confirm it.

3. **Answer the open questions.** After each addition aggountant lists what
   is missing, and the agent asks you. An empty answer does not mean "none":
   if you leave out the free meals, the per diem would be too large. So the
   agent asks even when the answer is zero. The questions include the purpose
   of the trip, where it started, how many free meals you had and whether
   the company paid for any of your meals on each travel day, whether the car
   was your own or under your control, and abroad, in which country each
   travel day ended.

4. **Look at the draft** at any time: ask the agent to show the expense
   claim. You see the trips, the travel days and per diems, the drives and
   mileage, the receipts and the total. Check the meals and the kilometres in
   particular, because aggountant computes the amounts from them.

### Questions, refusals and checks

The draft tells you three kinds of thing. An **open question** is an answer
you have not given yet. A **refusal** is something aggountant does not
approve as it stands. Both block approval of the claim, but not saving the
draft. A **check** blocks nothing, but it is shown in the claim and the agent
asks you about it.

| Code | What it means | What you do |
|---|---|---|
| `TRIP_STATUTORY_FIELD_MISSING` | the trip lacks its purpose, destination, mode of travel or (for drives) route | give the missing detail |
| `TRIP_START_PLACE_UNANSWERED` | it is not known whether the trip started from home, the workplace or elsewhere | say where you left from |
| `TRIP_DESTINATION_PLACES_UNANSWERED` | it is not known whether the trip went to a place in the register, such as a secondary workplace | say where the trip went; the register classifies the trip from it |
| `TRAVEL_FREE_MEALS_UNANSWERED`, `TRAVEL_COMPANY_PAID_MEALS_UNANSWERED` | a travel day lacks its free meals or company-paid meals | give the number or the amounts, zero too |
| `DRIVE_VEHICLE_OWNERSHIP_UNANSWERED` | it is not known whether the vehicle was yours or under your control | say so; mileage is tax-free only then |
| `LINE_RECEIPT_MISSING` | a receipt line has no receipt | give the receipt or say why there is none |
| `TRIP_OVERLAP` | the trip overlaps another trip of the same person | correct the times or remove one trip |
| `RECEIPT_ALREADY_BOOKED`, `RECEIPT_DUPLICATE` | the same receipt is already booked as a voucher or on another line | remove the line; a receipt is booked once |
| `ARTIFACT_MISSING` | a receipt or evidence file is not in the company home's attachment store, for example after restoring a backup | attach the file again and revise the line |
| `LINE_TRIP_COSTS_NOT_TAX_FREE` | the receipt belongs to a trip the register does not make a work trip | see the trip's own refusal |
| `TRIP_RECORDED_AFTERWARDS` | the trip was recorded only after the month following the trip | blocks nothing; the claim shows the marker |
| `DRIVE_KM_BELOW_DISTANCE`, `DRIVE_KM_ABOVE_ROUTE` | the kilometres driven differ a lot from the trip's distance | check the kilometres or give the route |
| `RECEIPT_OUTSIDE_TRIPS`, `RECEIPT_POSSIBLE_DUPLICATE` | a receipt is dated outside every trip, or looks like a copy of another | check that the cost is the company's and included once |

The allowance computation's own refusals, such as `ALLOWANCE_ABOVE_MAXIMUM`,
are explained in
[Compute the tax-free allowances of a trip](compute-travel-allowances.md#when-the-computation-refuses).

### A trip recorded afterwards

A trip that enters the draft only after the end of the month following the
month it ended in is marked as recorded afterwards. Claims are usually
collected monthly, so a trip recorded by the end of the next month is
ordinary. One recorded later rests on memory or a reconstruction made
afterwards. It is allowed, but the marker is shown in the claim. The law
gives no number for the limit; the limit follows the monthly claim cycle.

### Discarding a draft

If a draft is not needed, ask the agent to discard it and give the reason.
The draft is removed; its identifier, the hash of its last revision and the
reason stay in a log, its content does not.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant --json artifact attach --file receipt.pdf
aggountant --json claim trip add --new-for maija < trip.json
aggountant --json claim line add --claim maija-2026-03-10 < receipt.json
aggountant --json claim trip revise --claim maija-2026-03-10 --trip t1 < trip.json
aggountant claim show --claim maija-2026-03-10 --format markdown
aggountant --json claim discard --claim maija-2026-03-10 --reason "Made again"
```

`claim trip` and `claim line` add, revise and remove trips and receipt
lines, and every write answers with the draft's open questions. `claim show`
shows the draft as JSON or Markdown, and `claim list` lists the open drafts.
The input fields are shown by `aggountant claim trip add --help` and
`aggountant claim line add --help`.
</details>

## Done when

- The draft has no open questions and no refusals (`ready_for_approval` is
  `true`).
- You have looked at the claim and its checks.

## Limitations

- An expense claim cannot be approved or booked in aggountant yet. For now
  the allowances of a finished draft are recorded by hand, as
  [Compute the tax-free allowances of a trip](compute-travel-allowances.md)
  describes.
- The draft lives in the company home's `intake/expense-claims/` folder,
  which git does not track. It is not encrypted, as the personal-data store
  is not either in this version.
- [What is ready and what is not](../reference/status.md#receipts-and-entries)
  describes the situation.

## See also

- [Compute the tax-free allowances of a trip](compute-travel-allowances.md)
- [Record your regular workplace](record-regular-workplace.md)
- [Glossary](../reference/glossary.md)
