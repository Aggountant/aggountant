# Record your regular workplace

Tax-free travel allowances can be paid only for a trip that is not a trip
between home and the varsinainen työpaikka (regular workplace: the place where
you work permanently, for example your own office or a home office). So
aggountant needs, for everyone who files an expense claim, a dated record of
what their regular workplace is and of their relationship to the company. You
record it once, and the agent asks again only when there is reason to think it
changed.

If your company does not make expense claims, none of this is needed and
aggountant asks nothing.

## Before you start

- The person is in the employee register (`employee add`). The same register
  holds a board member and an owner who works without pay.
- You know the date from which the fact holds, for example the day the
  company's premises moved into your home.

## How to do it

1. **Tell the agent your situation in your own words.**

   > The company's premises have been in my home at Esimerkkikatu 1, Tampere,
   > since 1 January 2024. I am the managing director and own part of the
   > company.

2. **The agent proposes the entries.** It records the place (address and
   municipality) and proposes three dated facts: the relationship to the
   company (for example managing director), the pääomaosakkuus
   (shareholding), and the regular workplace with its basis. The basis comes
   from a closed list, and each basis refers to a passage of the Tax
   Administration's guidance, for example *the company's premises are in the
   home* (guidance 3.4) or *you work at the employer's premises weekly*
   (guidance 5.2.2, KHO 2017:178). If no basis fits, the agent writes the
   reason in its own words.

3. **Check and confirm.** Check that the date, the place and the basis are
   right. Only what you confirm is recorded, with who confirmed it and when.
   The program never infers these facts.

4. **Name your other permanent places**, if there are any:
   - another permanent place in the same municipality is another regular
     workplace;
   - premises of your own company or its group company in another
     municipality, where you work regularly, are a toissijainen työpaikka
     (secondary workplace: your company's other permanent premises in another
     municipality). A home or a customer's premises cannot be a secondary
     workplace;
   - a customer's premises you visit over a long period can be recorded as a
     tracked erityinen työntekemispaikka (special work site: a temporary place,
     such as at a customer's).

   A board member who does not otherwise work in the company is recorded as
   having no regular workplace.

Once the facts are in the register, the agent sees for each trip destination
whether it is a regular workplace (no tax-free allowances at all), a secondary
workplace (travel and lodging costs only, no per diem and no journey between
the lodging and the workplace) or a special work site (allowances as usual). A
place the register does not name is a special work site.

### When a fact changes or was wrong

A recorded fact is not edited. When something changes, for example the office
is given up, the agent records a correction that replaces the old fact, with
the reason. A correction can reach back in time. The old fact stays in the
history, so later you can see what the register said when an earlier claim was
approved.

A place is not edited either. If a place's address was wrong, the agent
records the place again under a new id, marks it as replacing the old one and
corrects the facts that named the old place to name the new one. The old place
can no longer be referred to after that.

Two contradicting facts for the same days cannot be recorded, for example two
primary regular workplaces. The agent then corrects the old fact instead.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant claimant place add < place.json
aggountant claimant add < fact.json
aggountant claimant correct < correction.json
aggountant claimant show --employee maija --date 2026-03-04
aggountant claimant show --employee maija --date 2026-03-04 --as-known-at 2026-01-15T00:00:00Z
aggountant claimant history --employee maija
aggountant claimant snapshot < trip.json
```

`claimant add` records one dated fact: a workplace role with its basis, a
relationship to the company, or the shareholding. `claimant show` shows what
was in force on a date, and with `--as-known-at` what was known about it at a
given moment. `claimant snapshot` classifies a trip's places and says what the
trip may be reimbursed for. `aggountant claimant add --help` lists the bases
and the guidance passages.
</details>

## Done when

- `claimant show` shows the person's relationship in force on the date and
  says the regular workplace is recorded (`regular_workplace_known` is
  `true`).

## Limitations

- Expense claims are not yet made with the program, so for now only the trip
  classification uses the register ([What is ready and what is not](../reference/status.md#receipts-and-entries)).
- Like the employee register, the register is stored unencrypted
  ([What is ready and what is not](../reference/status.md#tulorekisteri-and-employees)).

## If something goes wrong

- `REGULAR_WORKPLACE_UNKNOWN`, `CLAIMANT_FACT_CONFLICT` or another code →
  [Error codes](../reference/error-codes.md)

## See also

- [Report travel allowances to Tulorekisteri](report-travel-allowances.md)
- [Record a travel advance and its settlement](record-travel-advance.md)
- [Glossary](../reference/glossary.md)
