//! The generated employer's separate report validates against the official,
//! vendored `PayerSummaryReportsToIR` schema (`schemas/tulorekisteri/2025`).
//!
//! The validator below interprets the schema files themselves and supports only
//! the XSD subset those files use: global elements, complex types made of one
//! `xs:sequence` of `xs:element`s, and simple-type restrictions with length,
//! enumeration and the two pattern forms present. Any other construct it meets
//! fails the test instead of being skipped, so it cannot pass a record silently.
use aggountant_broker::tulorekisteri_xml::{payer_summary_record, PSR_NS};
use aggountant_domain::payroll::{
    EmployerReportAction, EmployerReportContent, EmployerSeparateReport, MonthPayments,
};
use chrono::{DateTime, NaiveDate, Utc};
use roxmltree::{Document, Node};
use serde_json::json;
use std::collections::HashMap;

const XS: &str = "http://www.w3.org/2001/XMLSchema";
type QName = (String, String);

struct Particle {
    name: QName,
    is_ref: bool,
    ty: Option<QName>,
    min: usize,
    max: usize,
}
struct Simple {
    base: QName,
    min_len: Option<usize>,
    max_len: Option<usize>,
    patterns: Vec<String>,
    enums: Vec<String>,
}
#[derive(Default)]
struct Schema {
    elements: HashMap<QName, QName>,
    complex: HashMap<QName, Vec<Particle>>,
    simple: HashMap<QName, Simple>,
}

fn schema_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas/tulorekisteri/2025")
}

fn qname(node: Node, value: &str) -> QName {
    let (prefix, local) = value.split_once(':').unwrap_or(("", value));
    let ns = node
        .lookup_namespace_uri(if prefix.is_empty() {
            None
        } else {
            Some(prefix)
        })
        .unwrap_or_else(|| panic!("unresolved prefix in {value}"));
    (ns.to_string(), local.to_string())
}

fn occurs(node: Node, attr: &str) -> usize {
    match node.attribute(attr) {
        None => 1,
        Some("unbounded") => usize::MAX,
        Some(n) => n.parse().unwrap(),
    }
}

impl Schema {
    fn load(files: &[&str]) -> Self {
        let mut schema = Schema::default();
        for file in files {
            let raw = std::fs::read_to_string(schema_dir().join(file)).unwrap();
            let text = raw.trim_start_matches('\u{feff}');
            let doc = Document::parse(text).unwrap();
            let root = doc.root_element();
            let tns = root.attribute("targetNamespace").unwrap().to_string();
            for item in root.children().filter(Node::is_element) {
                let name = || (tns.clone(), item.attribute("name").unwrap().to_string());
                match item.tag_name().name() {
                    "import" => {}
                    "element" => {
                        schema
                            .elements
                            .insert(name(), qname(item, item.attribute("type").unwrap()));
                    }
                    "complexType" => {
                        let kids: Vec<_> = item.children().filter(Node::is_element).collect();
                        assert!(
                            kids.len() == 1 && kids[0].has_tag_name((XS, "sequence")),
                            "unsupported complexType {:?}",
                            name()
                        );
                        let particles = kids[0]
                            .children()
                            .filter(Node::is_element)
                            .map(|el| {
                                assert!(el.has_tag_name((XS, "element")), "unsupported particle");
                                let (name, is_ref, ty) = match el.attribute("ref") {
                                    Some(r) => (qname(el, r), true, None),
                                    None => (
                                        // Local elements are unqualified (no elementFormDefault).
                                        (String::new(), el.attribute("name").unwrap().to_string()),
                                        false,
                                        Some(qname(el, el.attribute("type").unwrap())),
                                    ),
                                };
                                Particle {
                                    name,
                                    is_ref,
                                    ty,
                                    min: occurs(el, "minOccurs"),
                                    max: occurs(el, "maxOccurs"),
                                }
                            })
                            .collect();
                        schema.complex.insert(name(), particles);
                    }
                    "simpleType" => {
                        let r = item.first_element_child().unwrap();
                        assert!(
                            r.has_tag_name((XS, "restriction")),
                            "unsupported simpleType"
                        );
                        let mut s = Simple {
                            base: qname(r, r.attribute("base").unwrap()),
                            min_len: None,
                            max_len: None,
                            patterns: vec![],
                            enums: vec![],
                        };
                        for facet in r.children().filter(Node::is_element) {
                            let v = facet.attribute("value").unwrap().to_string();
                            match facet.tag_name().name() {
                                "minLength" => s.min_len = Some(v.parse().unwrap()),
                                "maxLength" => s.max_len = Some(v.parse().unwrap()),
                                "pattern" => s.patterns.push(v),
                                "enumeration" => s.enums.push(v),
                                other => panic!("unsupported facet {other}"),
                            }
                        }
                        schema.simple.insert(name(), s);
                    }
                    other => panic!("unsupported top-level {other}"),
                }
            }
        }
        schema
    }

    fn validate(&self, xml: &str) -> Result<(), String> {
        let doc = Document::parse(xml).map_err(|e| e.to_string())?;
        let root = doc.root_element();
        let key = (
            root.tag_name().namespace().unwrap_or_default().to_string(),
            root.tag_name().name().to_string(),
        );
        let ty = self
            .elements
            .get(&key)
            .ok_or_else(|| format!("no global element {key:?}"))?;
        self.element(root, ty, "")
    }

    fn element(&self, node: Node, ty: &QName, path: &str) -> Result<(), String> {
        let path = format!("{path}/{}", node.tag_name().name());
        if let Some(particles) = self.complex.get(ty) {
            if node
                .children()
                .any(|c| c.is_text() && !c.text().unwrap().trim().is_empty())
            {
                return Err(format!("{path}: text in complex element"));
            }
            let kids: Vec<_> = node.children().filter(Node::is_element).collect();
            let mut i = 0;
            for p in particles {
                let mut count = 0;
                while i < kids.len() {
                    let k = kids[i];
                    let name = (
                        k.tag_name().namespace().unwrap_or_default().to_string(),
                        k.tag_name().name().to_string(),
                    );
                    if name != p.name {
                        break;
                    }
                    if p.is_ref {
                        return Err(format!("{path}: unexpected {name:?} in an unsigned record"));
                    }
                    self.element(k, p.ty.as_ref().unwrap(), &path)?;
                    count += 1;
                    i += 1;
                }
                if count < p.min || count > p.max {
                    return Err(format!("{path}: {:?} occurs {count} times", p.name.1));
                }
            }
            if i != kids.len() {
                return Err(format!("{path}: unexpected {}", kids[i].tag_name().name()));
            }
            return Ok(());
        }
        if node.children().any(|c| c.is_element()) {
            return Err(format!("{path}: element children in simple element"));
        }
        let text = node.text().unwrap_or_default();
        self.simple_value(ty, text)
            .map_err(|e| format!("{path}: {e}"))
    }

    fn simple_value(&self, ty: &QName, v: &str) -> Result<(), String> {
        if ty.0 == XS {
            return builtin(&ty.1, v);
        }
        let s = self
            .simple
            .get(ty)
            .ok_or_else(|| format!("unknown type {ty:?}"))?;
        self.simple_value(&s.base, v)?;
        let len = v.chars().count();
        if s.min_len.is_some_and(|m| len < m) || s.max_len.is_some_and(|m| len > m) {
            return Err(format!("length {len} outside {:?}", ty.1));
        }
        if !s.enums.is_empty() {
            let same = |e: &String| match s.base.1.as_str() {
                "int" => e.parse::<i64>().ok() == v.parse::<i64>().ok(),
                _ => e == v,
            };
            if !s.enums.iter().any(same) {
                return Err(format!("{v:?} not in enumeration of {:?}", ty.1));
            }
        }
        if !s.patterns.is_empty() && !s.patterns.iter().any(|p| pattern(p, v)) {
            return Err(format!("{v:?} does not match pattern of {:?}", ty.1));
        }
        Ok(())
    }
}

fn builtin(name: &str, v: &str) -> Result<(), String> {
    let ok = match name {
        "string" => true,
        "normalizedString" => !v.contains(['\t', '\n', '\r']),
        "int" => v.parse::<i32>().is_ok(),
        "boolean" => matches!(v, "true" | "false" | "1" | "0"),
        "decimal" => rust_decimal::Decimal::from_str_exact(v).is_ok(),
        // The Incomes Register forbids a time zone on dates and requires one on times.
        "date" => NaiveDate::parse_from_str(v, "%Y-%m-%d").is_ok() && v.len() == 10,
        "dateTime" => DateTime::parse_from_rfc3339(v).is_ok(),
        other => return Err(format!("unsupported builtin xs:{other}")),
    };
    if ok {
        Ok(())
    } else {
        Err(format!("{v:?} is not a valid xs:{name}"))
    }
}

/// The only patterns in the vendored files: literals and `decimal2`.
fn pattern(p: &str, v: &str) -> bool {
    match p {
        "true" | "false" => p == v,
        r"[+-]?\d{1,13}(\.\d{1,2})?" => {
            let v = v.strip_prefix(['+', '-']).unwrap_or(v);
            let (int, frac) = v.split_once('.').map_or((v, None), |(a, b)| (a, Some(b)));
            (1..=13).contains(&int.len())
                && int.bytes().all(|b| b.is_ascii_digit())
                && frac.is_none_or(|f| {
                    (1..=2).contains(&f.len()) && f.bytes().all(|b| b.is_ascii_digit())
                })
        }
        other => panic!("unsupported pattern {other}"),
    }
}

fn schema() -> Schema {
    Schema::load(&[
        "IRCommonTypes.xsd",
        "PayerSummaryReportsToIRTypes.xsd",
        "PayerSummaryReportsToIR.xsd",
    ])
}

fn report() -> EmployerSeparateReport {
    serde_json::from_value(json!({
        "business_id":"1234567-1","month":"2026-09","content":{"kind":"no_wages_payable"},
        "action":{"kind":"new"},"contact_name":"Fictional Contact","contact_phone":"+358 40 1234567",
        "pension_insurances":[{"provider_code":46,"policy_number":"12-3456789"},{"provider_code":54,"policy_number":"A1"}]
    }))
    .unwrap()
}

fn at() -> DateTime<Utc> {
    "2026-10-02T09:30:00Z".parse().unwrap()
}

#[test]
fn every_report_shape_validates_against_the_official_schema() {
    let schema = schema();
    let mut shapes = vec![report()];
    let mut reimbursed = report();
    reimbursed.content =
        EmployerReportContent::for_payments(MonthPayments::TaxFreeReimbursementsOnly).unwrap();
    shapes.push(reimbursed.clone());
    for replaces_version in [Some(1), None] {
        let mut replacement = reimbursed.clone();
        replacement.action = EmployerReportAction::Replacement {
            ir_report_id: "22222222222233334444555555555555".into(),
            replaces_version,
        };
        shapes.push(replacement);
    }
    let mut no_pensions = report();
    no_pensions.pension_insurances.clear();
    shapes.push(no_pensions);
    for r in shapes {
        let xml = payer_summary_record(&r, at()).unwrap();
        assert!(xml.contains(PSR_NS));
        schema
            .validate(&xml)
            .unwrap_or_else(|e| panic!("{e}\n{xml}"));
    }
}

#[test]
fn the_validator_rejects_records_the_schema_forbids() {
    let schema = schema();
    let xml = payer_summary_record(&report(), at()).unwrap();
    let broken = [
        // Missing mandatory ReportDate.
        xml.replace("<ReportDate>2026-10-02</ReportDate>", ""),
        // Elements out of order.
        xml.replace(
            "<Source>aggountant</Source><DeliveryDataType>101</DeliveryDataType>",
            "<DeliveryDataType>101</DeliveryDataType><Source>aggountant</Source>",
        ),
        // Month outside MonthsType.
        xml.replace("<Month>9</Month>", "<Month>13</Month>"),
        // Amount with three decimals.
        payer_summary_record(
            &{
                let mut r = report();
                r.content =
                    EmployerReportContent::for_payments(MonthPayments::TaxFreeReimbursementsOnly)
                        .unwrap();
                r
            },
            at(),
        )
        .unwrap()
        .replace("<Amount>0.00</Amount>", "<Amount>0.000</Amount>"),
        // Unknown element.
        xml.replace("<Reports>", "<Extra>1</Extra><Reports>"),
    ];
    for b in broken {
        assert_ne!(b, xml);
        assert!(schema.validate(&b).is_err(), "accepted: {b}");
    }
}

#[test]
fn record_carries_the_documented_codes() {
    let xml = payer_summary_record(&report(), at()).unwrap();
    for needle in [
        "<DeliveryDataType>101</DeliveryDataType>",
        "<FaultyControl>2</FaultyControl>",
        "<ActionCode>1</ActionCode><ReportId>agg-er-202609</ReportId>",
        "<PaymentMonth><Month>9</Month><Year>2026</Year></PaymentMonth>",
        "<SummaryTransactionCode>101</SummaryTransactionCode></TransactionBasic>",
        "<PensionInsurance><PensionProvIdCode>46</PensionProvIdCode><PensionPolicyNo>12-3456789</PensionPolicyNo></PensionInsurance>",
    ] {
        assert!(xml.contains(needle), "{needle}\n{xml}");
    }
    let mut r = report();
    r.content =
        EmployerReportContent::for_payments(MonthPayments::TaxFreeReimbursementsOnly).unwrap();
    r.action = EmployerReportAction::Replacement {
        ir_report_id: "22222222222233334444555555555555".into(),
        replaces_version: Some(3),
    };
    let xml = payer_summary_record(&r, at()).unwrap();
    assert!(xml.contains("<ActionCode>2</ActionCode><IRReportId>22222222222233334444555555555555</IRReportId><ReportVersion>3</ReportVersion></ReportData>"));
    assert!(
        xml.contains("<SummaryTransactionCode>102</SummaryTransactionCode><Amount>0.00</Amount>")
    );
    // Pension insurances go only with "No wages payable".
    assert!(!xml.contains("PensionInsurances"));
    assert!(!xml.contains("<ReportId>"));
}
