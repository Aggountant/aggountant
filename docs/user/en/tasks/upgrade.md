# Upgrade aggountant

A new aggountant version is installed the same way as the first, and then the
skills are installed again. Commands and file formats may still change between
versions, so upgrade when no run is in progress.

## Before you start

- No month's close or filing is in progress. Ask the agent to check the state
  of the runs.
- You have a recent backup of the home
  ([Back up and restore your books](back-up-and-restore.md)).

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
   fixed copies of the chart of accounts and the accounts' VAT roles. The
   agent first runs a dry run, which only lists the missing files, and then
   the actual upgrade. The upgrade only adds missing files to
   `masterdata/reference/`: it does not switch the chart of accounts in use
   and does not change any existing file, so vouchers sealed earlier verify
   as before. If a file in the home differs from the one that comes with
   aggountant, the upgrade leaves it as it is and says so (`skipped`). A
   second run does nothing.

   The added files show up in `git status`. Commit them; that is the record
   of what the upgrade added.

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
  the one that comes with aggountant. If the company has its own chart of
  accounts for which aggountant brings no VAT roles, the upgrade cannot add
  them (`unavailable`). Do not copy files by hand; ask for help in the
  project's [issue tracker](https://github.com/aggountant/aggountant/issues).

## See also

- [Install aggountant and its skills](../start/install.md)
- [When something goes wrong](../troubleshooting.md)
