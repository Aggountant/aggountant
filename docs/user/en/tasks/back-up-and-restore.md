# Back up and restore your books

The most important part of your books, the sealed vouchers and archived
attachments, is not in git. That is why the whole company home is backed up
separately. On this page you make a backup and restore it when needed.

## Before you start

- No aggountant command is running. Ask the agent to wait until the copy has
  been taken.
- You have a place for the backup somewhere other than the original, for
  example an external disk or encrypted cloud storage that only you use.

## Make a backup

1. **Copy the whole home directory**, including the files excluded from git:
   the voucher database (`fiscal.sqlite` and the files next to it whose names
   start with `fiscal.sqlite-`), the attachments (`artifacts/`) and the
   personal data (`masterdata/pii.sqlite`). You can do it yourself or ask the
   agent:

   > Back up the books home to the disk /Volumes/Backup.

2. **Keep the copy** for as long as the Accounting Act requires records to be
   kept. The copy contains personal data, so keep it for your own use only.

aggountant has no backup or export command of its own. The backup is an
ordinary copy of the directory.

## Restore a backup

1. **Move the damaged home aside**, do not delete it: it may still be useful.
2. **Copy the backup** into the home's place.
3. **Ask the agent to check the voucher chain.**

   > Check the voucher chain.

4. **Check that everything is there.** The agent shows the state of the
   periods and the number of the last voucher. If the copy is older than the
   latest entries, record the missing transactions again from their sources.
   If a return was filed with an authority after the backup was taken, tell
   the agent before anything is sent again.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant voucher verify
aggountant period show --period 2026-03
aggountant config show
```
</details>

## Done when

`aggountant voucher verify` replies, for example:

```
chain ok — 42 vouchers, head #42
```

## If something goes wrong

- **The chain check still fails after restoring.** Try an older copy:
  [When something goes wrong](../troubleshooting.md#the-chain-check-fails).

## See also

- [The company home: what is stored where](../concepts/company-home.md)
- [Upgrade aggountant](upgrade.md)
