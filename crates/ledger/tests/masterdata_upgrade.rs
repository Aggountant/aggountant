//! `home upgrade`: an existing home takes in the masterdata a newer binary needs.
//!
//! The fixture is a home in the layout an older `init` wrote (chart, version and
//! README, but no `chart-snapshots/`, `vat-account-roles.json` or
//! `settlement-account-roles.json`) holding
//! vouchers it sealed. The upgrade must add exactly the missing files, keep the
//! active chart and every existing file byte-identical, and leave the sealed
//! chain verifying.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use aggountant_domain::PolicySet;
use aggountant_ledger::{Ledger, SealRequest, UpgradeResource, UpgradeState};
use aggountant_model::{AccountId, Currency, Date, Money, Posting, Side, VoucherDraft};
use sha2::{Digest, Sha256};

fn reference_dir(home: &Path) -> PathBuf {
    home.join("masterdata").join("reference")
}

/// Every file under `dir` with its bytes, so a test can prove nothing changed.
fn tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.insert(
                    path.strip_prefix(dir).unwrap().to_path_buf(),
                    fs::read(&path).unwrap(),
                );
            }
        }
    }
    files
}

fn seal_purchase(ledger: &Ledger, key: &str, amount: &str) {
    let eur = Currency::new("EUR").unwrap();
    let posting = |account: &str, side| Posting {
        account: AccountId::new(account).unwrap(),
        amount: Money::parse(amount, eur.clone()).unwrap(),
        side,
        vat_code: None,
    };
    let date = Date::parse("2026-03-14").unwrap();
    let draft = VoucherDraft::new(
        date,
        date,
        "Esimerkki Oy hosting invoice".into(),
        vec![posting("7690", Side::Debit), posting("1910", Side::Credit)],
        None,
    )
    .unwrap();
    ledger
        .seal(
            &draft,
            &SealRequest {
                natural_key: key.into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();
}

/// A home as an older binary left it: vouchers sealed against the bundled chart,
/// then only the files that older `init` wrote.
fn older_binary_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    {
        let ledger = Ledger::init(home.path()).unwrap();
        seal_purchase(&ledger, "esimerkki:receipt-1", "100.00");
        seal_purchase(&ledger, "esimerkki:receipt-2", "240.00");
    }
    let reference = reference_dir(home.path());
    fs::remove_file(reference.join("vat-account-roles.json")).unwrap();
    fs::remove_file(reference.join("settlement-account-roles.json")).unwrap();
    fs::remove_dir_all(reference.join("chart-snapshots")).unwrap();
    home
}

fn states(upgrade: &aggountant_ledger::MasterdataUpgrade) -> Vec<(UpgradeResource, UpgradeState)> {
    upgrade
        .items
        .iter()
        .map(|item| (item.resource, item.state))
        .collect()
}

#[test]
fn older_home_is_upgraded_and_its_sealed_vouchers_still_verify() {
    let home = older_binary_home();
    let reference = reference_dir(home.path());
    let before_reference = tree(&reference);
    let before_home = tree(home.path());
    let err = Ledger::open(home.path()).map(|_| ()).unwrap_err();
    assert_eq!(
        err.error_code(),
        "MASTERDATA_CORRUPT",
        "the newer binary refuses it"
    );

    // Dry-run lists the missing masterdata and writes nothing anywhere in the home.
    let plan = Ledger::upgrade_masterdata(home.path(), false).unwrap();
    assert!(!plan.applied);
    assert_eq!(
        states(&plan),
        vec![
            (UpgradeResource::ChartSnapshot, UpgradeState::Create),
            (UpgradeResource::VatAccountRoles, UpgradeState::Create),
            (
                UpgradeResource::SettlementAccountRoles,
                UpgradeState::Create
            ),
        ]
    );
    assert!(
        plan.masterdata_check.is_none(),
        "the plan predicts an opening home"
    );
    assert_eq!(tree(home.path()), before_home);

    // Apply adds exactly the planned files, which are what a fresh init writes.
    let applied = Ledger::upgrade_masterdata(home.path(), true).unwrap();
    assert_eq!(
        states(&applied),
        vec![
            (UpgradeResource::ChartSnapshot, UpgradeState::Created),
            (UpgradeResource::VatAccountRoles, UpgradeState::Created),
            (
                UpgradeResource::SettlementAccountRoles,
                UpgradeState::Created
            ),
        ]
    );
    let planned: Vec<_> = plan.items.iter().map(|i| &i.path).collect();
    let created: Vec<_> = applied.items.iter().map(|i| &i.path).collect();
    assert_eq!(planned, created);
    assert!(applied.masterdata_check.is_none());

    let after = tree(&reference);
    let fresh = tempfile::tempdir().unwrap();
    Ledger::init(fresh.path()).unwrap();
    let fresh_reference = tree(&reference_dir(fresh.path()));
    for path in &created {
        let path = Path::new(path.as_str());
        assert_eq!(
            after[path],
            fresh_reference[path],
            "{} matches init",
            path.display()
        );
    }
    // Existing files, the active chart among them, are byte-identical, and the
    // only new files are the created ones.
    for (path, bytes) in &before_reference {
        assert_eq!(&after[path], bytes, "{} unchanged", path.display());
    }
    assert_eq!(after.len(), before_reference.len() + created.len());
    // Nothing outside masterdata/reference/ was touched (the fiscal store included).
    let after_home = tree(home.path());
    for (path, bytes) in &before_home {
        assert_eq!(&after_home[path], bytes, "{} unchanged", path.display());
    }

    // A second run is a no-op.
    let again = Ledger::upgrade_masterdata(home.path(), true).unwrap();
    assert!(again.items.iter().all(|i| i.state == UpgradeState::Present));
    assert_eq!(tree(home.path()), after_home);

    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(
        ledger.chart_version().unwrap(),
        applied.active_chart_version,
        "active chart choice unchanged"
    );
    let report = ledger.verify().unwrap();
    assert!(report.ok);
    assert_eq!(report.voucher_count, 2);
}

#[test]
fn a_differing_existing_file_is_reported_and_left_untouched() {
    let home = older_binary_home();
    let reference = reference_dir(home.path());
    // The company's own role file: same meaning, different bytes.
    let fresh = tempfile::tempdir().unwrap();
    Ledger::init(fresh.path()).unwrap();
    let roles: serde_json::Value = serde_json::from_slice(
        &fs::read(reference_dir(fresh.path()).join("vat-account-roles.json")).unwrap(),
    )
    .unwrap();
    let company_roles = serde_json::to_vec(&roles).unwrap();
    fs::write(reference.join("vat-account-roles.json"), &company_roles).unwrap();

    let applied = Ledger::upgrade_masterdata(home.path(), true).unwrap();
    assert_eq!(
        states(&applied),
        vec![
            (UpgradeResource::ChartSnapshot, UpgradeState::Created),
            (UpgradeResource::VatAccountRoles, UpgradeState::Differs),
            (
                UpgradeResource::SettlementAccountRoles,
                UpgradeState::Created
            ),
        ]
    );
    assert!(applied.items[1].detail.is_some());
    assert_eq!(
        fs::read(reference.join("vat-account-roles.json")).unwrap(),
        company_roles
    );
    assert!(applied.masterdata_check.is_none());
    Ledger::open(home.path()).unwrap().verify().unwrap();
}

#[test]
fn roles_are_not_added_for_a_company_chart_the_bundle_does_not_know() {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    let reference = reference_dir(home.path());
    // A company chart (renamed edition) without role metadata.
    let chart_path = reference.join("chart-of-accounts.json");
    let mut chart: serde_json::Value =
        serde_json::from_slice(&fs::read(&chart_path).unwrap()).unwrap();
    chart["chart"] = serde_json::json!("esimerkki-chart");
    let canonical = aggountant_wire::canonical_bytes(&chart).unwrap();
    let version = format!("esimerkki-chart@{}", hex::encode(Sha256::digest(canonical)));
    fs::write(&chart_path, serde_json::to_vec_pretty(&chart).unwrap()).unwrap();
    fs::write(reference.join("chart_version"), format!("{version}\n")).unwrap();
    fs::remove_file(reference.join("vat-account-roles.json")).unwrap();
    fs::remove_file(reference.join("settlement-account-roles.json")).unwrap();
    let before = tree(&reference);

    let applied = Ledger::upgrade_masterdata(home.path(), true).unwrap();
    assert_eq!(applied.active_chart_version, version);
    assert_eq!(
        states(&applied),
        vec![
            (UpgradeResource::ChartSnapshot, UpgradeState::Present),
            (UpgradeResource::VatAccountRoles, UpgradeState::Unavailable),
            (
                UpgradeResource::SettlementAccountRoles,
                UpgradeState::Unavailable
            ),
        ]
    );
    assert!(applied.items[2]
        .detail
        .as_deref()
        .unwrap()
        .contains("no entry for the active chart"));
    assert_eq!(
        applied.masterdata_check.as_ref().unwrap().error_code(),
        "MASTERDATA_CORRUPT"
    );
    assert_eq!(tree(&reference), before, "nothing written");
}

/// A home made by the `init` just before settlement roles existed: everything
/// else is in place and the home opens, but payments are not yet recognised.
#[test]
fn settlement_roles_are_added_to_a_home_that_has_everything_else() {
    let home = tempfile::tempdir().unwrap();
    {
        let ledger = Ledger::init(home.path()).unwrap();
        seal_purchase(&ledger, "esimerkki:receipt-1", "100.00");
    }
    let reference = reference_dir(home.path());
    fs::remove_file(reference.join("settlement-account-roles.json")).unwrap();
    let before_home = tree(home.path());
    Ledger::open(home.path()).expect("the home opens without the role file");

    let plan = Ledger::upgrade_masterdata(home.path(), false).unwrap();
    assert_eq!(
        states(&plan),
        vec![
            (UpgradeResource::ChartSnapshot, UpgradeState::Present),
            (UpgradeResource::VatAccountRoles, UpgradeState::Present),
            (
                UpgradeResource::SettlementAccountRoles,
                UpgradeState::Create
            ),
        ]
    );
    assert_eq!(plan.items[2].path, "settlement-account-roles.json");
    assert!(plan.masterdata_check.is_none());
    assert_eq!(tree(home.path()), before_home, "dry-run writes nothing");

    let applied = Ledger::upgrade_masterdata(home.path(), true).unwrap();
    assert_eq!(
        applied.items[2].state,
        UpgradeState::Created,
        "{:?}",
        applied.items
    );
    assert!(applied.masterdata_check.is_none());
    let after_home = tree(home.path());
    for (path, bytes) in &before_home {
        assert_eq!(&after_home[path], bytes, "{} unchanged", path.display());
    }
    assert_eq!(after_home.len(), before_home.len() + 1);
    let fresh = tempfile::tempdir().unwrap();
    Ledger::init(fresh.path()).unwrap();
    let file = Path::new("settlement-account-roles.json");
    assert_eq!(
        tree(&reference)[file],
        tree(&reference_dir(fresh.path()))[file],
        "matches init"
    );

    let again = Ledger::upgrade_masterdata(home.path(), true).unwrap();
    assert!(again.items.iter().all(|i| i.state == UpgradeState::Present));
    assert_eq!(tree(home.path()), after_home);

    let ledger = Ledger::open(home.path()).unwrap();
    let version = ledger.chart_version().unwrap();
    assert!(ledger.settlement_role_table().resolve(&version).is_some());
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn a_differing_settlement_role_file_is_reported_and_left_untouched() {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    let path = reference_dir(home.path()).join("settlement-account-roles.json");
    let mut roles: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    roles["charts"][0]["accounts"] = serde_json::json!(["1910"]);
    let company = serde_json::to_vec(&roles).unwrap();
    fs::write(&path, &company).unwrap();

    let applied = Ledger::upgrade_masterdata(home.path(), true).unwrap();
    assert_eq!(applied.items[2].state, UpgradeState::Differs);
    assert!(applied.items[2].detail.is_some());
    assert!(applied.masterdata_check.is_none());
    assert_eq!(fs::read(&path).unwrap(), company);
}

#[test]
fn an_unreadable_existing_snapshot_refuses_the_whole_upgrade() {
    let home = older_binary_home();
    let reference = reference_dir(home.path());
    let snapshots = reference.join("chart-snapshots");
    fs::create_dir(&snapshots).unwrap();
    fs::write(snapshots.join(format!("{}.json", "0".repeat(64))), b"{}").unwrap();
    let before = tree(home.path());

    for apply in [false, true] {
        assert!(Ledger::upgrade_masterdata(home.path(), apply).is_err());
    }
    assert_eq!(tree(home.path()), before, "nothing written");
}

#[test]
fn an_uninitialized_home_is_refused_without_creating_it() {
    let parent = tempfile::tempdir().unwrap();
    let home = parent.path().join("missing");
    let err = Ledger::upgrade_masterdata(&home, true).unwrap_err();
    assert_eq!(err.error_code(), "MASTERDATA_NOT_INITIALIZED");
    assert!(!home.exists());
}

/// The VAT role file exactly as `init` wrote it before the default chart had an
/// `eu-goods` reverse-charge target.
const ROLES_BEFORE_EU_GOODS: &str =
    include_str!("../data/retired-vat-account-roles/before-eu-goods.json");

#[test]
fn an_unedited_role_file_from_an_earlier_release_gains_only_the_new_target() {
    let home = tempfile::tempdir().unwrap();
    {
        let ledger = Ledger::init(home.path()).unwrap();
        seal_purchase(&ledger, "esimerkki:receipt-1", "100.00");
    }
    let reference = reference_dir(home.path());
    let roles_path = reference.join("vat-account-roles.json");
    fs::write(&roles_path, ROLES_BEFORE_EU_GOODS).unwrap();
    let before_home = tree(home.path());

    let plan = Ledger::upgrade_masterdata(home.path(), false).unwrap();
    assert_eq!(
        states(&plan),
        vec![
            (UpgradeResource::ChartSnapshot, UpgradeState::Present),
            (UpgradeResource::VatAccountRoles, UpgradeState::Update),
            (
                UpgradeResource::SettlementAccountRoles,
                UpgradeState::Present
            ),
        ]
    );
    assert!(plan.masterdata_check.is_none());
    assert_eq!(tree(home.path()), before_home, "dry-run writes nothing");

    let applied = Ledger::upgrade_masterdata(home.path(), true).unwrap();
    assert_eq!(applied.items[1].state, UpgradeState::Updated);
    assert!(applied.masterdata_check.is_none());

    // Only the role file changed, and it now equals what a fresh init writes.
    let after_home = tree(home.path());
    let roles_file = Path::new("masterdata/reference/vat-account-roles.json");
    assert_eq!(after_home.len(), before_home.len());
    for (path, bytes) in &before_home {
        if path != roles_file {
            assert_eq!(&after_home[path], bytes, "{} unchanged", path.display());
        }
    }
    let fresh = tempfile::tempdir().unwrap();
    Ledger::init(fresh.path()).unwrap();
    assert_eq!(
        fs::read(&roles_path).unwrap(),
        fs::read(reference_dir(fresh.path()).join("vat-account-roles.json")).unwrap()
    );

    // The difference is exactly one added target: eu-goods on the domestic pair.
    let old: serde_json::Value = serde_json::from_str(ROLES_BEFORE_EU_GOODS).unwrap();
    let mut new: serde_json::Value =
        serde_json::from_slice(&fs::read(&roles_path).unwrap()).unwrap();
    let targets = new["charts"][0]["reverse_charge_targets"]
        .as_array_mut()
        .unwrap();
    let added = targets
        .iter()
        .position(|t| t["scope"] == "eu-goods")
        .expect("eu-goods target added");
    assert_eq!(
        targets.remove(added),
        serde_json::json!({"scope":"eu-goods","input_account":"1763","output_account":"2939"})
    );
    assert_eq!(new, old, "nothing else changed");

    let again = Ledger::upgrade_masterdata(home.path(), true).unwrap();
    assert!(again.items.iter().all(|i| i.state == UpgradeState::Present));
    assert_eq!(tree(home.path()), after_home);
    assert!(Ledger::open(home.path()).unwrap().verify().unwrap().ok);
}

#[test]
fn an_edited_role_file_from_an_earlier_release_is_left_untouched() {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    let roles_path = reference_dir(home.path()).join("vat-account-roles.json");
    // Same roles, but no longer the bytes the release wrote: a company's own file.
    let edited = ROLES_BEFORE_EU_GOODS.replace("  ", "    ");
    fs::write(&roles_path, &edited).unwrap();

    for apply in [false, true] {
        let upgrade = Ledger::upgrade_masterdata(home.path(), apply).unwrap();
        assert_eq!(upgrade.items[1].state, UpgradeState::Differs);
        assert!(upgrade.masterdata_check.is_none());
    }
    assert_eq!(fs::read_to_string(&roles_path).unwrap(), edited);
}
