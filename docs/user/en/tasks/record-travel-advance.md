# Record a travel advance and its settlement

When the company pays you or an employee money before a trip, the advance
(matkaennakko) is the company's receivable from that person until the
expense claim for the trip settles it. The agent records the advance, the
claim and the offset between them as ordinary vouchers, and only the
difference is paid or returned.

## Before you start

- The advance payment is on the bank statement, or you know when and how much
  was paid.
- When you settle the advance, you have the travel or expense claim for the
  trip.

## How to do it

1. **Tell the agent about the advance** when you import the statement it was
   paid on.

   > The €500 payment to Maija Meikäläinen on 3 March is a travel advance for
   > the Tampere trip.

   The agent books the row to 1730 Muut lyhytaikaiset saamiset (other
   short-term receivables) without VAT and writes the person and the trip in
   the description. Check in the statement preview that the row is a
   receivable and not an expense.

2. **Ask the agent to record the expense claim** once the trip is done.

   > Record the expense claim for Maija's Tampere trip and offset the advance.

   The agent records the whole claim as if there were no advance: receipts
   through the receipt route and mileage and per diems as a manual entry, all
   against 2740 Muut lyhytaikaiset velat (other short-term liabilities). It
   then prepares the offset as a voucher of its own: debit 2740 and credit
   1730, at most the open part of the advance and at most the claim total.
   Check the offset's amount and date before you approve. The date should be
   the day the difference is paid or returned, because the offset date is the
   Tulorekisteri payment date, and on the same day the offset and the bank
   payment fall in the same month. If there is no difference, the date is the
   day the claim is approved.

3. **Pay or return the difference**, and the agent records it from the next
   statement. When the company pays the difference, the contra account is
   2740. When the person returns an advance that was too large, or the trip
   was cancelled, the contra account is 1730.

4. **Report the allowances to Tulorekisteri** as usual. The advance is not
   reported. The claim's mileage and per diems are reported in full, even when
   part of them was paid by the offset, and the offset date is the payment
   date. [Report travel allowances to Tulorekisteri](report-travel-allowances.md)
   tells how to make the report.

For example, the advance is €500.00 and the claim €620.00. The offset moves
€500.00 from 2740 to 1730, and the company pays Maija the difference of
€120.00 the same day. At the end 1730 is zero, and 2740 holds nothing for this
claim.

Settle the advance soon after the trip. The agent reminds you of an advance
still open a month after the trip. As a shareholder, always settle or return
the advance before the end of the year: an unused advance cannot be turned
into a shareholder loan afterwards, and an advance that is not returned can be
a hidden dividend.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant import camt seal --file tiliote-2026-03.xml --dry-run < kohdistukset.json
aggountant capture-receipt record --dry-run < kuitti.json
aggountant voucher validate < kululasku.json
aggountant voucher seal < kululasku.json
aggountant voucher validate < ennakon-kuittaus.json
aggountant voucher seal < ennakon-kuittaus.json
```

`import camt seal` books the advance payment and later the difference, each as
a statement row with its own contra account. After the preview the agent seals
the statement you approved, as in [importing the bank statement](import-bank-statement.md).

`capture-receipt record` books the trip's receipts as a liability on 2740, as
in [recording receipts](record-receipts.md).

The first `voucher seal` books mileage and per diems to 7300 Matkakulut
(travel expenses) and 7350 Päivärahat (per diems) with the travel claim
attached. The second books the offset of the advance, also with the travel
claim attached.
</details>

## Done when

- The balance of 1730 no longer contains this advance, and 2740 has no unpaid
  part of this claim.
- The agent has told you the voucher number of the offset.

## Limitations

- aggountant does not yet have the expense claim or the travel advance as a
  feature of its own, so the agent puts the vouchers together by hand and you
  check the amounts.
  [What is ready and what is not](../reference/status.md#receipts-and-entries)
  describes the situation.

## See also

- [Report travel allowances to Tulorekisteri](report-travel-allowances.md)
- [Record a sales invoice, payroll or another manual entry](record-other-entries.md)
- [Import the bank statement](import-bank-statement.md)
