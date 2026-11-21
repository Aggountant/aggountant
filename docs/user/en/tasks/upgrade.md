# Upgrade aggountant

A new aggountant version is installed the same way as the first, and then the
skills are installed again. Commands and file formats may still change between
versions, so upgrade when no run is in progress.

## Before you start

- No month's close or filing is in progress. Ask the agent to check the state
  of the runs.
- You have a backup of the home taken just before the upgrade
  ([Back up and restore your books](back-up-and-restore.md)). A new version
  can change the format of the voucher database (`fiscal.sqlite`) the first
  time it opens the home, and the older version cannot open the database
  after that. The backup is then the only way back to the older version.
  The project's
  [changelog](https://github.com/aggountant/aggountant/blob/main/CHANGELOG.md)
  says for each version whether this happens and which commands changed.

## How to do it

1. **Ask the agent to upgrade.**

   > Upgrade aggountant to the latest version and reinstall the skills.

2. **The agent installs the new version** with the same command as the first
   time ([Install aggountant and its skills](../start/install.md)).

3. **The agent installs the skills again.** This is needed every time,
   because the skills are written for the commands of the same version. Old
   skills would steer the agent to commands that may no longer exist.

4. **Restart the agent** so that it reads the new skills.

5. **The agent upgrades the home's master data.** A new version can need
   master data that the old version did not write when it set up the home:
   fixed copies of the chart of accounts, the accounts' VAT roles and the
   settlement account roles (`settlement-account-roles.json`: the bank and tax
   accounts VAT is paid and refunded through). Without the settlement roles
   the home opens, but the VAT return preview asks for every VAT payment to be
   checked. The agent first runs a dry run, which only lists the missing files, and then
   the actual upgrade. The upgrade adds missing files to
   `masterdata/reference/` and does not switch the chart of accounts in use.
   Of the existing files it updates only the VAT roles
   (`vat-account-roles.json`), and only when the file is byte for byte what
   an earlier aggountant version wrote and the new version merely adds
   reverse-charge account pairs to it (for example for EU goods purchases,
   `eu-goods`). Every old role stays, so vouchers sealed earlier verify as
   before (`"action": "update"`). If a file in the home differs from the one
   that comes with aggountant, for example the company's own chart import,
   the upgrade leaves it as it is and says so (`skipped`). A second run does
   nothing.

   The added and updated files show up in `git status`. Commit them; that is
   the record of what the upgrade changed.

6. **If a run was in progress after all** and continuing it gives the code
   `WORKFLOW_VERSION_MISMATCH`, the run's definition changed between versions.
   The agent can move the run to the new version. Moving changes the run's
   version permanently, so the agent asks your permission.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant run list
cargo install --git https://github.com/aggountant/aggountant aggountant --locked
aggountant --version
aggountant skill install
aggountant home upgrade --dry-run
aggountant home upgrade --apply
aggountant run migrate --run-id cp-… --confirm
```
</details>

## Done when

- `aggountant --version` shows the new version.
- `aggountant home upgrade --dry-run` would add nothing more, and
  `masterdata_opens` is `true`.
- `aggountant voucher verify` replies `chain ok`.

## Limitations

- The upgrade does not fix or replace a file that differs in the home from
  the one that comes with aggountant or an earlier version of it. If the home's chart of accounts is one
  for which aggountant brings no VAT roles or settlement roles, the upgrade
  cannot add them (`unavailable`). Do not copy files by hand; ask for help in the
  project's [issue tracker](https://github.com/aggountant/aggountant/issues).

## See also

- [Install aggountant and its skills](../start/install.md)
- [When something goes wrong](../troubleshooting.md)
