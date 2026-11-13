# Practise your first month

On this page you record the fictional company Esimerkki Oy's March 2026 from
start to finish: you import the bank statement, record a receipt, look at the
month's report, close the month and look at the VAT return figures. Along the
way you see what the agent does on its own, what it asks you and what you
approve. The exercise takes about half an hour.

You practise in a separate **practice home** that has nothing to do with your
company's real books. That matters, because two things you learn on this page
are permanent: a sealed voucher cannot be changed, and a closed month cannot
be reopened. In a practice home that does not matter, and when you are done
you can delete the whole directory.

## Before you start

- aggountant and its skills are installed
  ([Install aggountant and its skills](install.md)).
- An agent, for example Claude Code, is running in a terminal.
- Copy the practice material from [`docs/user/practice/`](../../practice/README.md)
  into a folder of your own, for example `~/aggountant-practice-material/`. You
  can also ask the agent to fetch it from GitHub. The material has three files,
  all in Finnish, as real Finnish material would be:
  - `tiliote-2026-03.xml`: Esimerkki Oy's bank statement for March
  - `lasku-pilvipalvelu-2026-03.txt`: an invoice from Pilvipalvelu Oy for
    server rental
  - `kuitti-toimistotarvike-2026-03.txt`: a receipt for office supplies.

  The invoice and the receipt are text files so that they can be read
  directly. Real receipts are usually PDF files or images, and the agent
  handles them the same way.

The requests to the agent below are examples. You can say the same thing in
your own words. The agent's replies vary from one run to the next, but
aggountant's own output is the same as on this page.

## 1. Set up a practice home

Tell the agent:

> Set up a practice home in ~/aggountant-practice. This is for practice only:
> use --home ~/aggountant-practice in every command and do not run the company
> setup.

The agent runs `aggountant --home ~/aggountant-practice init`, and the program
replies:

```
initialized /…/aggountant-practice — chart_version fi-general-2026@…, 98 accounts
```

The practice home now has the default chart of accounts with 98 accounts and
an empty voucher store. The account names are in Finnish, as in every Finnish
chart of accounts.

Why `--home` in every command? If you have already set your real books as the
default home, the program would otherwise use that. `--home` overrides
everything else, so the exercise cannot touch your real books by accident. If
in doubt, ask the agent to run `aggountant config show`. It tells you which
home is in use and where that came from.

## 2. Import the bank statement

Tell the agent:

> Import the bank statement tiliote-2026-03.xml from the practice material and
> propose the entries. Pilvipalvelu Oy's invoice is in the same folder.

The agent reads the statement. Reading does not record anything yet:

```
parsed "FI2112345600000785" — 2 transactions
```

The statement has three rows, but the program takes only two of them: the
€50.00 card reservation on 31 March has not yet been confirmed by the bank,
and it is not recorded until it has gone through.

The agent cannot know everything about the second row, so it asks, for
example:

> On 20 March, €2,500.00 came in from Maija Meikäläinen with the message
> "Osakepääoma" (share capital). Is this the share capital payment?

Answer:

> Yes, I paid the share capital when the company was founded.

Then the agent shows its proposal for the whole statement. A good approval
request shows every row:

| Date | Counterparty | Amount | Entry | VAT |
|---|---|---|---|---|
| 14 Mar | Pilvipalvelu Oy | −€124.00 | 7690 Atk-laite- ja ohjelmistokulut (IT hardware and software) | domestic purchase 25.5 %, invoice date 11 Mar |
| 20 Mar | Maija Meikäläinen | +€2,500.00 | 2000 Osake- tai osuuspääoma (share capital) | no VAT |

Check two things before you approve. Is the account right? And was the VAT
rate chosen from the invoice date rather than the payment date? The rate
depends on when the purchase was made. When everything is in order, say:

> I approve, record it.

The agent seals the whole statement at once, and the program replies:

```
sealed 2 vouchers from "FI2112345600000785" (attachment "…")
```

The books now have two vouchers. The original statement file has been archived
and attached to both. The program worked out the VAT on the server rental
itself: €98.80 as an expense and €25.20 as VAT receivable. The agent used the
invoice as the source for the invoice date and the VAT. In this version, a
voucher recorded from the bank statement has the statement attached, not the
invoice
([What is ready and what is not](../reference/status.md#purchase-invoice-paid-from-the-bank-account)).

Now try what happens when the same statement is imported again:

> Import the same statement once more with the same entries.

The program recognises the transactions it has already sealed and does not
record them a second time. It prints the same line as before, but there are
still two vouchers. You can see that from the chain check
(`aggountant voucher verify`).

## 3. Record a receipt

On 22 March, Maija bought printer paper and envelopes for €31.00 and paid with
her own card. That is why the purchase is not on the company's bank statement.
Tell the agent:

> Record the receipt kuitti-toimistotarvike-2026-03.txt.

The agent archives the receipt and notices that the purchase is not on the
bank statement. It asks how the purchase was paid. Tell it:

> I paid with my own card.

The agent proposes recording the purchase as a debt from the company to you
and shows the program's preview:

| Account | Debit | Credit |
|---|---|---|
| 7700 Toimistokulut (office expenses) | €24.70 | |
| 1763 Arvonlisäverosaaminen (VAT receivable) | €6.30 | |
| 2740 Muut lyhytaikaiset velat (other short-term liabilities) | | €31.00 |

The preview does not store anything. The program built the VAT line itself
from its VAT rate table by the receipt's date. When you approve, the agent
seals the voucher exactly like this:

```
sealed receipt voucher #3 (replayed=false)
```

The approval is bound to what you were shown. If the receipt details, the
proposal or the chart of accounts had changed between the preview and the
sealing, the program would have refused, and the agent would have had to show
a new preview. Read more: [Sealing and approval](../concepts/sealing-and-approval.md).

Ask the agent to record the same receipt again. The program returns the same
voucher number 3 (`replayed=true`) and does not create a new one. So the same
receipt cannot end up in the books twice by accident.

## 4. See how the month went

Tell the agent:

> Make a monthly report for March in HTML.

The agent runs `report statements`, and you get a file to open in a browser.
The report itself is in Finnish. It shows:

- **the month's result** −€123.50 (server rental €98.80 and office expenses
  €24.70)
- **Pankkitili** (bank account) €2,376.00
- **balance sheet totals** €2,407.50 on both sides.

Compare the bank account balance with the closing balance on the statement.
Both are €2,376.00, so every transaction on the statement has been recorded.
This is the single most important check before closing a month.

At the top of the report it says "Kattavuus: varmistamaton" (completeness:
unverified). The program cannot know whether all the material has been
recorded. You and the agent judge that against the sources, as you just did.

## 5. Close the month

Tell the agent:

> Close March.

The agent first checks that every transaction on the statement is recorded,
the receipt is attached, the bank balance matches and there are no open
obligations. Then it starts the close:

```
run "cp-…" — "awaiting_gate" ["2026-03"]
```

The month is now in the state `closing`. Nothing more can be recorded in it,
and the close waits for your approval. The agent tells you what approving
means: the month is locked permanently, and corrections found later are
recorded in the next month. Say:

> I approve the close.

```
run "cp-…" — "completed" ["2026-03"]
```

March is closed (`sealed`). Try what happens when you still try to record
something in it:

> Record the same office supplies receipt again under a new identifier, so I
> can see what happens.

The preview succeeds, but the sealing does not:

```
PERIOD_SEALED: period 2026-03 is Sealed; corrections go to the next open period
```

This is what the Finnish Accounting Act (kirjanpitolaki) requires: a closed
period is not changed, and a correction is recorded in an open period. If
you notice a gap before you approve a close, reject it: the month returns to
open ([Periods, closing and the fiscal year](../concepts/periods.md)).

## 6. Look at the VAT return figures

Tell the agent:

> Show the figures for March's VAT return.

The agent runs `file-vat preview`. The preview only reads the closed month's
books and does not contact anything:

| Field | Amount |
|---|---|
| 307 Kohdekauden vähennettävä vero (deductible tax for the period) | €31.50 |
| 308 Maksettava tai palautukseen oikeuttava vero (tax payable or refundable) | −€31.50 |
| To pay | €0.00 |

The deductible tax of €31.50 is the server rental's €25.20 plus the
receipt's €6.30. There were no sales, so there is nothing to pay.

In the exercise you send nothing. In your real books you would enter these
fields in OmaVero, or let the agent send the return directly if direct filing
is set up.

## What you learned

- The agent does on its own whatever the material is enough for, and asks you
  only what it cannot know, such as where €2,500 came from or how a receipt
  was paid.
- Nothing is stored before you approve it, and you always approve what you
  were shown.
- A sealed voucher and a closed month are permanent. A mistake is corrected by
  a new voucher in the next period.
- Comparing the bank account balance with the statement tells you whether the
  month has been recorded in full.
- VAT is calculated from the closed month's entries, and the rate is chosen
  from the date of the invoice or receipt.

## Next

You can delete the practice home when you no longer need it: it is just the
directory `~/aggountant-practice`. Start your real books on
[Set up your company's books](set-up-company.md).
