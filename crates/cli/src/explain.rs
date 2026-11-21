//! `aggountant explain <CODE>` and the `hint` field of the error JSON.
//!
//! The one source is the user guide's error-code table
//! (`docs/user/{fi,en}/reference/error-codes.md`), bundled into the binary and
//! parsed here, so an explanation the program prints is the guide's text and
//! the two cannot drift. The hint is Finnish: the Finnish guide is the
//! original, and the skills, which the hint relieves of their code
//! explanations, are Finnish too. `explain --lang en` prints the English entry.

use clap::ValueEnum;
use serde_json::{json, Value};

use crate::CliError;

const GUIDE_FI: &str = include_str!("../../../docs/user/fi/reference/error-codes.md");
const GUIDE_EN: &str = include_str!("../../../docs/user/en/reference/error-codes.md");

/// Base URL of the published guide; `docs/user/AGENTS.md` ("The site") gives
/// the path mapping: Finnish at the root, English under `/en/`.
const SITE: &str = "https://aggountant.dev";
/// The page's own location below the language root, for resolving links.
const PAGE_DIR: &str = "reference";

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Lang {
    Fi,
    En,
}

impl Lang {
    fn as_str(self) -> &'static str {
        match self {
            Lang::Fi => "fi",
            Lang::En => "en",
        }
    }

    fn guide(self) -> &'static str {
        match self {
            Lang::Fi => GUIDE_FI,
            Lang::En => GUIDE_EN,
        }
    }

    fn site_root(self) -> String {
        match self {
            Lang::Fi => SITE.to_string(),
            Lang::En => format!("{SITE}/en"),
        }
    }
}

/// One row of the guide's code table, cells as written in the Markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) code: String,
    pub(crate) what_happened: String,
    pub(crate) what_to_do: String,
}

/// The table: its two column headings after the code column, and its rows.
#[derive(Debug)]
pub(crate) struct Table {
    pub(crate) headings: [String; 2],
    pub(crate) entries: Vec<Entry>,
}

fn cells(line: &str) -> Option<Vec<String>> {
    let inner = line.trim().strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(|c| c.trim().to_string()).collect())
}

/// Parses the first Markdown table whose rows start with a backticked code.
/// The guide's table format is a contract with this parser; the tests below
/// hold both languages to it.
pub(crate) fn parse(markdown: &str) -> Table {
    let mut lines = markdown.lines().peekable();
    let mut headings = [String::new(), String::new()];
    let mut entries = Vec::new();
    while let Some(line) = lines.next() {
        let is_header = lines
            .peek()
            .is_some_and(|next| next.trim_start().starts_with("|---"));
        if !is_header {
            continue;
        }
        let Some(header) = cells(line) else { continue };
        lines.next();
        let mut rows = Vec::new();
        while let Some(row) = lines.peek().and_then(|l| cells(l)) {
            lines.next();
            rows.push(row);
        }
        let parsed: Vec<Entry> = rows
            .into_iter()
            .filter_map(|row| match row.as_slice() {
                [code, happened, todo] => Some(Entry {
                    code: code.strip_prefix('`')?.strip_suffix('`')?.to_string(),
                    what_happened: happened.clone(),
                    what_to_do: todo.clone(),
                }),
                _ => None,
            })
            .collect();
        if !parsed.is_empty() && header.len() == 3 {
            headings = [header[1].clone(), header[2].clone()];
            entries = parsed;
            break;
        }
    }
    Table { headings, entries }
}

/// Turns the guide's relative Markdown links into "text (site URL)" so the
/// text stands alone outside the guide.
pub(crate) fn render_links(text: &str, lang: Lang) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        let parsed = after
            .find(']')
            .filter(|&close| after[close..].starts_with("](") && !after[..close].contains('['))
            .and_then(|close| {
                let target_start = close + 2;
                after[target_start..]
                    .find(')')
                    .map(|end| (&after[..close], &after[target_start..target_start + end]))
            });
        match parsed {
            Some((label, target)) => {
                out.push_str(&rest[..open]);
                out.push_str(label);
                out.push_str(" (");
                out.push_str(&site_url(target, lang));
                out.push(')');
                rest = &after[label.len() + 2 + target.len() + 1..];
            }
            None => {
                out.push_str(&rest[..=open]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Maps a link relative to `reference/error-codes.md` to its page on the site.
fn site_url(target: &str, lang: Lang) -> String {
    if target.starts_with("http://") || target.starts_with("https://") {
        return target.to_string();
    }
    let (path, anchor) = match target.split_once('#') {
        Some((path, anchor)) => (path, Some(anchor)),
        None => (target, None),
    };
    let mut segments: Vec<&str> = PAGE_DIR.split('/').collect();
    for part in path.split('/') {
        match part {
            ".." => {
                segments.pop();
            }
            "." | "" => {}
            other => segments.push(other),
        }
    }
    let mut page = segments.join("/");
    if let Some(stem) = page.strip_suffix(".md") {
        page = stem.to_string();
    }
    if page == "README" {
        page = "guide".to_string();
    }
    let mut url = format!("{}/{}/", lang.site_root(), page);
    if let Some(anchor) = anchor {
        url.push('#');
        url.push_str(anchor);
    }
    url
}

fn lookup(code: &str, lang: Lang) -> Option<Entry> {
    parse(lang.guide())
        .entries
        .into_iter()
        .find(|entry| entry.code == code)
}

/// The short `hint` for the error JSON: the Finnish guide's "what to do" cell
/// for `code`, or `None` when the guide has no entry for it.
pub(crate) fn hint(code: &str) -> Option<String> {
    lookup(code, Lang::Fi).map(|entry| render_links(&entry.what_to_do, Lang::Fi))
}

fn page_url(lang: Lang) -> String {
    format!("{}/{PAGE_DIR}/error-codes/", lang.site_root())
}

pub(crate) fn execute(code: Option<&str>, lang: Lang) -> Result<Value, CliError> {
    let table = parse(lang.guide());
    let Some(code) = code else {
        let codes: Vec<Value> = table
            .entries
            .iter()
            .map(|entry| {
                json!({
                    "code": entry.code,
                    "what_happened": render_links(&entry.what_happened, lang),
                })
            })
            .collect();
        return Ok(json!({
            "schema_version": 1,
            "lang": lang.as_str(),
            "guide_url": page_url(lang),
            "codes": codes,
        }));
    };
    if code.is_empty()
        || !code
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
    {
        return Err(CliError::user(
            "INVALID_INPUT",
            format!(
                "INVALID_INPUT: `{code}` is not an error code; codes are upper-case \
                 letters, digits and underscores, as in the `error_code` field"
            ),
        ));
    }
    let Some(entry) = table.entries.iter().find(|entry| entry.code == code) else {
        let blockers = if code.starts_with("VERO_") {
            format!(
                "; Vero.fi readiness blockers are explained on {}/{PAGE_DIR}/vero-fi-blockers/",
                lang.site_root()
            )
        } else {
            String::new()
        };
        return Err(CliError::user(
            "ERROR_CODE_UNKNOWN",
            format!(
                "ERROR_CODE_UNKNOWN: the guide has no entry for {code}{blockers}; the error's \
                 `message` gives the cause, and `aggountant explain` lists the codes it explains"
            ),
        ));
    };
    Ok(json!({
        "schema_version": 1,
        "lang": lang.as_str(),
        "code": entry.code,
        "what_happened": render_links(&entry.what_happened, lang),
        "what_to_do": render_links(&entry.what_to_do, lang),
        "headings": table.headings,
        "guide_url": page_url(lang),
    }))
}

pub(crate) fn human(value: &Value) -> String {
    let str_of = |v: &Value| v.as_str().unwrap_or("?").to_string();
    if let Some(codes) = value["codes"].as_array() {
        return codes
            .iter()
            .map(|c| format!("{} — {}", str_of(&c["code"]), str_of(&c["what_happened"])))
            .collect::<Vec<_>>()
            .join("\n");
    }
    format!(
        "{}\n{}: {}\n{}: {}\n{}",
        str_of(&value["code"]),
        str_of(&value["headings"][0]),
        str_of(&value["what_happened"]),
        str_of(&value["headings"][1]),
        str_of(&value["what_to_do"]),
        str_of(&value["guide_url"]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn both_languages_parse_with_the_same_codes_in_the_same_order() {
        let fi = parse(GUIDE_FI);
        let en = parse(GUIDE_EN);
        assert!(
            fi.entries.len() >= 30,
            "fi table parsed {} rows",
            fi.entries.len()
        );
        let fi_codes: Vec<_> = fi.entries.iter().map(|e| &e.code).collect();
        let en_codes: Vec<_> = en.entries.iter().map(|e| &e.code).collect();
        assert_eq!(fi_codes, en_codes);
        assert_eq!(fi.headings, ["Mitä tapahtui", "Mitä tehdään"]);
        assert_eq!(en.headings, ["What happened", "What is done"]);
    }

    /// Every row of the guide's table is a three-cell row this parser reads;
    /// a malformed row (an unescaped `|`, a missing cell) would otherwise drop
    /// out of `explain` silently.
    #[test]
    fn every_table_row_parses() {
        for (name, guide) in [("fi", GUIDE_FI), ("en", GUIDE_EN)] {
            let rows = guide.lines().filter(|l| l.starts_with("| `")).count();
            let table = parse(guide);
            assert_eq!(table.entries.len(), rows, "{name}: a row did not parse");
            for entry in &table.entries {
                assert!(!entry.what_happened.is_empty(), "{name} {}", entry.code);
                assert!(!entry.what_to_do.is_empty(), "{name} {}", entry.code);
                assert!(
                    !render_links(&entry.what_to_do, Lang::Fi).contains("]("),
                    "{name} {}: unrendered link",
                    entry.code
                );
            }
        }
    }

    /// The guide documents only codes the program can produce: every code
    /// appears as a string literal in some crate's source.
    #[test]
    fn every_guide_code_exists_in_the_source() {
        let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut source = String::new();
        let mut stack = vec![crates];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    if path
                        .file_name()
                        .is_some_and(|n| n != "target" && n != "tests")
                    {
                        stack.push(path);
                    }
                } else if path.extension().is_some_and(|e| e == "rs") {
                    source.push_str(&std::fs::read_to_string(&path).unwrap());
                }
            }
        }
        // This module's own tests name codes; exclude it so it proves nothing.
        let own = include_str!("explain.rs");
        let source = source.replace(own, "");
        for entry in parse(GUIDE_FI).entries {
            assert!(
                source.contains(&format!("\"{}\"", entry.code)),
                "the guide documents {} but no crate source names it",
                entry.code
            );
        }
    }

    #[test]
    fn links_become_site_urls() {
        assert_eq!(
            render_links("[Päivitä aggountant](../tasks/upgrade.md)", Lang::Fi),
            "Päivitä aggountant (https://aggountant.dev/tasks/upgrade/)"
        );
        assert_eq!(
            render_links("see [x](status.md#vat-return) now", Lang::En),
            "see x (https://aggountant.dev/en/reference/status/#vat-return) now"
        );
        assert_eq!(
            render_links("a [b] c ([d](../troubleshooting.md))", Lang::Fi),
            "a [b] c (d (https://aggountant.dev/troubleshooting/))"
        );
    }

    #[test]
    fn hint_is_the_finnish_what_to_do_cell() {
        assert_eq!(
            hint("PERIOD_SEALED").as_deref(),
            Some("kirjaa seuraavalle avoimelle kaudelle")
        );
        assert_eq!(hint("NO_SUCH_CODE"), None);
    }
}
