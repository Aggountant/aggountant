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

5. **If a run was in progress after all** and continuing it gives the code
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
aggountant run migrate --run-id cp-… --confirm
```
</details>

## Done when

- `aggountant --version` shows the new version.
- `aggountant voucher verify` replies `chain ok`.

## Limitations

- If the new version expects information in the home that an old home does
  not have, there is no separate upgrade command yet. Do not copy files by
  hand; ask for help in the project's
  [issue tracker](https://github.com/aggountant/aggountant/issues).

## See also

- [Install aggountant and its skills](../start/install.md)
- [When something goes wrong](../troubleshooting.md)
