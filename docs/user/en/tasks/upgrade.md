# Upgrade aggountant

To upgrade, you install the new version of the program, install the skills
again, and the agent upgrades the company home's master data. Commands and
file formats may still change between versions, so upgrade when no run is in
progress.

## Before you start

- No month's close or filing is in progress. Ask the agent to check the state
  of the runs.
- You have [backed up the home](back-up-and-restore.md) just before the
  upgrade. A new version can change the format of the voucher database
  (`fiscal.sqlite`) the first time it opens the home, and the older version
  cannot open the database after that. The backup is then the only way back
  to the older version. The project's
  [changelog](https://github.com/aggountant/aggountant/blob/main/CHANGELOG.md)
  says for each version whether this happens and which commands changed.

## How to do it

1. **Ask the agent to upgrade.**

   > Upgrade aggountant to the latest version and reinstall the skills.

2. **The agent installs the new version** the same way as
   [the first time](../start/install.md). The installer fetches the latest
   release from GitHub and installs it in `~/.cargo/bin`. That is the same
   directory a version compiled from source goes into, so the installer also
   replaces a version you first installed from source. If there is no
   prebuilt package for your machine, the agent compiles the new version from
   source.

3. **The agent installs the skills again.** The skills are written for the
   commands of the same version, so they are installed with every upgrade.
   Old skills would steer the agent to commands that may no longer exist.

4. **Restart the agent** so that it reads the new skills.

5. **The agent upgrades the home's master data.** A new version can need
   master data that the old version did not write when it set up the home:
   fixed copies of the chart of accounts, the accounts' VAT roles and the
   settlement account roles, which say through which bank and tax accounts
   VAT is paid and refunded. Without the settlement roles the home opens, but
   the VAT return preview asks for every VAT payment to be checked.

   The agent first runs a dry run, which only lists the missing files, and
   then the actual upgrade. The upgrade adds the missing files. If the VAT
   roles file has not been edited and the new version brings reverse-charge
   account pairs to it, the upgrade extends it. The old roles stay, so
   vouchers sealed earlier verify as before. The upgrade does not switch the
   chart of accounts in use. Any other file whose content in the home differs
   from the one that comes with aggountant is left as it is, and the upgrade
   says so. This happens, for example, when the company has imported its own
   chart of accounts.

6. **Commit the changes.** The added and updated files show up in
   `git status`, and once committed they are the record of what the upgrade
   changed.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant run list
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aggountant/aggountant/releases/latest/download/aggountant-installer.sh | sh
cargo install --git https://github.com/aggountant/aggountant aggountant --locked
aggountant --version
aggountant skill install
aggountant home upgrade --dry-run
aggountant home upgrade --apply
aggountant run migrate --run-id cp-… --confirm
```

The `curl` command installs the prebuilt package. `cargo install` compiles
the program from source and is needed only when there is no prebuilt package.

`home upgrade --dry-run` lists what the upgrade would add and writes nothing.

`home upgrade --apply` adds the missing files to `masterdata/reference/`. The
settlement account roles are in `settlement-account-roles.json`. Of the
existing files the upgrade changes only the VAT roles
(`vat-account-roles.json`), and only when the file is byte for byte what an
earlier aggountant version wrote and the new version adds reverse-charge
account pairs to it, for example for EU goods purchases (`eu-goods`). In the
output this file has `"action": "update"`. Files left as they are appear
under `skipped`. A second run does nothing.

`run migrate` is needed only if a run was left in progress before the
upgrade.
</details>

## Done when

- `aggountant --version` shows the new version.
- `aggountant home upgrade --dry-run` would add nothing more, and
  `masterdata_opens` is `true`.
- `aggountant voucher verify` replies `chain ok`.

## Limitations

- The upgrade does not fix or replace a file it left as it was, and it
  cannot add VAT roles or settlement roles that aggountant does not bring for
  the home's chart of accounts. Details are under
  [Company home and setup](../reference/status.md#company-home-and-setup).

## If something goes wrong

- If a run was in progress after all and continuing it gives the code
  `WORKFLOW_VERSION_MISMATCH`, the run's definition changed between versions.
  The agent can move the run to the new version. Moving changes the run's
  version permanently, so the agent asks your permission.

## See also

- [Install aggountant and its skills](../start/install.md)
- [When something goes wrong](../troubleshooting.md)
