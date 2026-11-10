use std::fs;

use aggountant_model::Date;
use aggountant_process::{
    AddCompanyKnowledge, CompanyKnowledgeSource, CompanyKnowledgeStatus, ProcessEngine,
    RetireCompanyKnowledge, UpdateCompanyKnowledge,
};

fn date(value: &str) -> Date {
    Date::parse(value).unwrap()
}

fn source(reference: &str, value: &str) -> CompanyKnowledgeSource {
    CompanyKnowledgeSource {
        reference: reference.to_string(),
        date: date(value),
    }
}

fn add() -> AddCompanyKnowledge {
    AddCompanyKnowledge {
        id: "hosting-vat-treatment".to_string(),
        statement: "Hosting invoices are handled as reverse-charge purchases.".to_string(),
        scope: "Applies to Example Hosting Ltd invoices for the managed hosting service."
            .to_string(),
        source: source("user decision in conversation 2026-09-12", "2026-09-12"),
    }
}

#[test]
fn create_read_search_and_reopen_preserve_source() {
    let home = tempfile::tempdir().unwrap();
    let engine = ProcessEngine::open(home.path());
    let created = engine.add_company_knowledge(&add()).unwrap();
    assert_eq!(created.revisions.len(), 1);
    assert_eq!(created.revisions[0].status, CompanyKnowledgeStatus::Active);
    assert_eq!(
        created.revisions[0].source.reference,
        "user decision in conversation 2026-09-12"
    );

    let fresh = ProcessEngine::open(home.path());
    let shown = fresh
        .show_company_knowledge("hosting-vat-treatment", None)
        .unwrap();
    assert_eq!(shown.statement, add().statement);
    assert_eq!(fresh.list_company_knowledge(false).unwrap().len(), 1);
    assert_eq!(
        fresh
            .search_company_knowledge("EXAMPLE HOSTING", false)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        fresh
            .search_company_knowledge("conversation 2026", false)
            .unwrap()
            .len(),
        1
    );
    assert!(fresh
        .search_company_knowledge("payroll", false)
        .unwrap()
        .is_empty());
}

#[test]
fn update_and_retire_keep_an_understandable_history() {
    let home = tempfile::tempdir().unwrap();
    let engine = ProcessEngine::open(home.path());
    engine.add_company_knowledge(&add()).unwrap();
    let updated = engine
        .update_company_knowledge(
            "hosting-vat-treatment",
            &UpdateCompanyKnowledge {
                statement: "Hosting invoices are ordinary Finnish purchases.".to_string(),
                scope: "Applies from the supplier's Finnish establishment change.".to_string(),
                source: source("supplier notice notice.pdf", "2026-10-01"),
                reason: "Supplier began invoicing from its Finnish establishment.".to_string(),
            },
        )
        .unwrap();
    assert_eq!(updated.revisions.len(), 2);
    assert_eq!(updated.revisions[0].statement, add().statement);
    assert_eq!(updated.revisions[1].revision, 2);
    assert!(updated.revisions[1]
        .reason
        .as_deref()
        .unwrap()
        .contains("Finnish establishment"));

    let retired = engine
        .retire_company_knowledge(
            "hosting-vat-treatment",
            &RetireCompanyKnowledge {
                source: source("contract termination confirmation", "2026-12-31"),
                reason: "The hosting contract ended.".to_string(),
            },
        )
        .unwrap();
    assert_eq!(retired.revisions.len(), 3);
    assert_eq!(retired.revisions[2].status, CompanyKnowledgeStatus::Retired);
    assert_eq!(
        engine
            .show_company_knowledge("hosting-vat-treatment", Some(1))
            .unwrap()
            .statement,
        add().statement
    );
    assert!(engine.list_company_knowledge(false).unwrap().is_empty());
    assert_eq!(engine.list_company_knowledge(true).unwrap().len(), 1);
    assert!(engine
        .search_company_knowledge("hosting", false)
        .unwrap()
        .is_empty());
    assert_eq!(
        engine
            .search_company_knowledge("hosting", true)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn malformed_writes_leave_existing_data_byte_identical() {
    let home = tempfile::tempdir().unwrap();
    let engine = ProcessEngine::open(home.path());
    engine.add_company_knowledge(&add()).unwrap();
    let path = home
        .path()
        .join("knowledge/company/hosting-vat-treatment.json");
    let before = fs::read(&path).unwrap();

    let bad = UpdateCompanyKnowledge {
        statement: " Changed after validation".to_string(),
        scope: "All invoices".to_string(),
        source: source("user", "2026-09-13"),
        reason: "Correction".to_string(),
    };
    let error = engine
        .update_company_knowledge("hosting-vat-treatment", &bad)
        .unwrap_err();
    assert_eq!(error.error_code(), "INVALID_KNOWLEDGE");
    assert_eq!(fs::read(&path).unwrap(), before);

    let missing_reason = RetireCompanyKnowledge {
        source: source("user", "2026-09-13"),
        reason: String::new(),
    };
    assert!(engine
        .retire_company_knowledge("hosting-vat-treatment", &missing_reason)
        .is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn homes_are_isolated_without_company_identity_metadata() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    ProcessEngine::open(first.path())
        .add_company_knowledge(&add())
        .unwrap();
    assert!(ProcessEngine::open(second.path())
        .list_company_knowledge(true)
        .unwrap()
        .is_empty());
    assert!(!first
        .path()
        .join("process/user-knowledge-company.json")
        .exists());
    assert!(!second
        .path()
        .join("process/user-knowledge-company.json")
        .exists());
}

#[test]
fn duplicate_missing_reactivation_and_corrupt_operations_are_clear() {
    let home = tempfile::tempdir().unwrap();
    let engine = ProcessEngine::open(home.path());
    engine.add_company_knowledge(&add()).unwrap();
    assert_eq!(
        engine
            .add_company_knowledge(&add())
            .unwrap_err()
            .error_code(),
        "INVALID_KNOWLEDGE"
    );
    assert_eq!(
        engine
            .show_company_knowledge("missing-fact", None)
            .unwrap_err()
            .error_code(),
        "KNOWLEDGE_NOT_FOUND"
    );
    engine
        .retire_company_knowledge(
            "hosting-vat-treatment",
            &RetireCompanyKnowledge {
                source: source("user", "2026-09-13"),
                reason: "No longer used.".to_string(),
            },
        )
        .unwrap();
    let reactivated = engine
        .update_company_knowledge(
            "hosting-vat-treatment",
            &UpdateCompanyKnowledge {
                statement: "Relevant again under a new contract.".to_string(),
                scope: "The replacement hosting contract.".to_string(),
                source: source("user statement: replacement contract", "2026-09-14"),
                reason: "The company resumed the service.".to_string(),
            },
        )
        .unwrap();
    assert_eq!(reactivated.revisions.len(), 3);
    assert_eq!(
        reactivated.revisions[2].status,
        CompanyKnowledgeStatus::Active
    );

    fs::write(
        home.path()
            .join("knowledge/company/hosting-vat-treatment.json"),
        b"not json",
    )
    .unwrap();
    let error = engine.list_company_knowledge(true).unwrap_err();
    assert_eq!(error.error_code(), "KNOWLEDGE_HISTORY_CORRUPT");
    assert_eq!(error.kind(), "system");
}

#[test]
fn stale_temp_files_do_not_hide_committed_knowledge() {
    let home = tempfile::tempdir().unwrap();
    let engine = ProcessEngine::open(home.path());
    engine.add_company_knowledge(&add()).unwrap();
    let path = home
        .path()
        .join("knowledge/company/hosting-vat-treatment.json");
    let before = fs::read(&path).unwrap();
    fs::write(path.with_extension("json.tmp"), b"partial replacement").unwrap();
    fs::write(home.path().join("knowledge/company/.DS_Store"), b"metadata").unwrap();

    assert_eq!(engine.list_company_knowledge(false).unwrap().len(), 1);
    assert_eq!(
        engine
            .search_company_knowledge("hosting", false)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    engine
        .update_company_knowledge(
            "hosting-vat-treatment",
            &UpdateCompanyKnowledge {
                statement: "Updated after an interrupted attempt.".to_string(),
                scope: "Example Hosting Ltd invoices.".to_string(),
                source: source("user statement: retry", "2026-09-13"),
                reason: "Completed the interrupted update.".to_string(),
            },
        )
        .unwrap();
    assert_eq!(
        engine
            .show_company_knowledge("hosting-vat-treatment", None)
            .unwrap()
            .revision,
        2
    );
}

#[test]
fn valid_reformatting_is_accepted_but_semantic_history_corruption_is_not() {
    let home = tempfile::tempdir().unwrap();
    let engine = ProcessEngine::open(home.path());
    let document = engine.add_company_knowledge(&add()).unwrap();
    let path = home
        .path()
        .join("knowledge/company/hosting-vat-treatment.json");
    let compact = serde_json::to_vec(&document).unwrap();
    fs::write(&path, [&compact[..], b"\n"].concat()).unwrap();
    assert_eq!(
        engine
            .show_company_knowledge("hosting-vat-treatment", None)
            .unwrap()
            .revision,
        1
    );

    let mut malformed = serde_json::to_value(&document).unwrap();
    malformed["revisions"][0]["revision"] = serde_json::json!(2);
    fs::write(&path, serde_json::to_vec_pretty(&malformed).unwrap()).unwrap();
    let before = fs::read(&path).unwrap();
    let error = engine
        .update_company_knowledge(
            "hosting-vat-treatment",
            &UpdateCompanyKnowledge {
                statement: "Would overwrite corrupt history.".to_string(),
                scope: "Any scope.".to_string(),
                source: source("user statement: invalid attempt", "2026-09-13"),
                reason: "Should not land.".to_string(),
            },
        )
        .unwrap_err();
    assert_eq!(error.error_code(), "KNOWLEDGE_HISTORY_CORRUPT");
    assert_eq!(fs::read(path).unwrap(), before);
}
