# Record receipts and purchase invoices

An ordinary purchase receipt or invoice is recorded as a voucher of its own,
and the original file is attached to it. This route is used especially for
purchases that do not show on the company's bank statement, such as those
paid with your own card.

## Before you start

- The receipt or invoice is a file (PDF, image or other), or the agent finds
  it in the place described in the home's `AGENTS.md`.
- You know how the purchase was paid: from the company's account, with the
  company's card or with your own money.

## How to do it

1. **Ask the agent.**

   > Record the receipt kuitti-toimistotarvike-2026-03.pdf. I paid with my own
   > card.

2. **The agent archives the file.** The program stores it and recognises the
   same file later, so a duplicate receipt is noticed.

3. **The agent previews the entry.** It keeps the details read from the
   receipt (amount, receipt date, description) apart from its own proposal
   (expense account, payment account, VAT treatment). The program builds the
   VAT lines from its VAT rate table by the receipt's date. The preview stores
   nothing. If a decisive detail is missing, the agent does not guess but
   asks: [How the VAT treatment is decided](../concepts/vat-treatment.md).

4. **Check and approve.** The agent shows the source details, the proposal and
   the resulting postings. For example, a software licence of €124.00
   (including VAT 25.5 %) paid from the company's account:

   | Account | Debit | Credit |
   |---|---|---|
   | 7690 Atk-laite- ja ohjelmistokulut (IT hardware and software) | €98.80 | |
   | 1763 Arvonlisäverosaaminen (VAT receivable) | €25.20 | |
   | 1910 Pankkitili (bank account) | | €124.00 |

   Check that the amount and date match the receipt, the expense belongs to
   the company, the account and VAT treatment are right, and the payment
   account matches how the purchase was paid. A purchase paid with your own
   money is recorded as a debt from the company to you (2740 Muut
   lyhytaikaiset velat, other short-term liabilities).

5. **The agent seals the voucher** exactly as you saw it. If something changed
   in between, the program refuses and the agent shows a new preview.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant artifact attach --file receipt.pdf
aggountant voucher list --attachment-ref <identifier>
aggountant capture-receipt record --dry-run < receipt.json
aggountant capture-receipt record --approved-preparation-hash <hash> < receipt.json
```
</details>

## Done when

- The program replies `sealed receipt voucher #N (replayed=false)`.
- Recording the same receipt again returns the same voucher
  (`replayed=true`) and does not create a new one.

## When a receipt cannot be found

When the bank statement has a payment for which no receipt can be found, the
agent looks for it in the places described in the home's `AGENTS.md`. If the
receipt is not found, the agent names the transaction as open and tells you
what is missing. Get the receipt from the seller or from your online shop
account and give it to the agent. aggountant's own automatic receipt search is
not in use yet, but the agent can search with its own tools wherever you have
given it access.

## Limitations

- The route handles domestic purchases, reverse-charge purchases and
  VAT-exempt purchases. It does not handle partial deduction rights or
  imports.
- Outside consultation for an unusual receipt is not in use. The agent asks
  you or your bookkeeper.
- If the receipt was paid from the company's account, the same payment is
  also on the bank statement, and the statement row must be recorded in the
  import. Watch out for double booking:
  [Import the bank statement](import-bank-statement.md#limitations).

More detail: [What is ready and what is not](../reference/status.md#receipts-and-entries).

## If something goes wrong

- **`CAPTURE_PREPARATION_CHANGED`.** Something changed after the preview. The
  agent makes a new preview, and you approve again.
- **`PERIOD_CLOSING` or `PERIOD_SEALED`.** The month is closed or its close is
  in progress. Record it in the next open month.
- Other codes: [Error codes](../reference/error-codes.md).

## See also

- [How the VAT treatment is decided](../concepts/vat-treatment.md)
- [Sealing and approval](../concepts/sealing-and-approval.md)
- [Import the bank statement](import-bank-statement.md)
