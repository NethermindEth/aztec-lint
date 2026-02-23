use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use aztec_lint_core::diagnostics::Confidence;
use aztec_lint_core::lints::{LintLifecycleState, LintSpec, all_lints};
use serde_json::json;

use crate::common::{
    DynError, ensure_no_unknown_options, parse_flags_and_options, read_text_file, workspace_root,
    write_text_file,
};

pub fn run(args: &[String]) -> Result<(), DynError> {
    let (mut flags, options) = parse_flags_and_options(args)?;
    let check = flags.remove("check");
    ensure_no_unknown_options(&flags, &options)?;

    let root = workspace_root()?;
    let portal_root = root.join("docs/portal");

    let generated = build_generated_files(&portal_root);

    if check {
        verify_generated_files(&portal_root, &generated)?;
        println!("docs-portal check: generated docs are in sync");
        return Ok(());
    }

    for (path, contents) in &generated {
        write_text_file(path, contents)?;
    }
    remove_stale_generated_files(&portal_root, &generated)?;
    println!("docs-portal: updated {}", portal_root.display());
    Ok(())
}

fn build_generated_files(portal_root: &Path) -> BTreeMap<PathBuf, String> {
    let mut files = BTreeMap::<PathBuf, String>::new();
    let active = all_lints()
        .iter()
        .filter(|lint| lint.lifecycle.is_active())
        .collect::<Vec<_>>();

    files.insert(portal_root.join("index.md"), render_index(&active));
    files.insert(
        portal_root.join("search-index.json"),
        render_search_index(&active),
    );

    for lint in active {
        let page = portal_root
            .join("lints")
            .join(format!("{}.md", lint.id.to_ascii_lowercase()));
        files.insert(page, render_lint_page(lint));
    }
    files
}

fn verify_generated_files(
    portal_root: &Path,
    generated: &BTreeMap<PathBuf, String>,
) -> Result<(), DynError> {
    for (path, expected) in generated {
        if !path.is_file() {
            return Err(format!(
                "missing generated portal file '{}'; run docs-portal",
                path.display()
            )
            .into());
        }
        let actual = read_text_file(path)?;
        if &actual != expected {
            return Err(format!(
                "stale generated portal file '{}'; run docs-portal",
                path.display()
            )
            .into());
        }
    }

    let existing = list_generated_files(portal_root)?;
    let expected_paths = generated
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    for path in existing {
        if !expected_paths.contains(&path) {
            return Err(format!(
                "stale portal file '{}' is not in generated set",
                path.display()
            )
            .into());
        }
    }
    Ok(())
}

fn remove_stale_generated_files(
    portal_root: &Path,
    generated: &BTreeMap<PathBuf, String>,
) -> Result<(), DynError> {
    let existing = list_generated_files(portal_root)?;
    let expected_paths = generated
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    for path in existing {
        if expected_paths.contains(&path) {
            continue;
        }
        fs::remove_file(path)?;
    }
    Ok(())
}

fn list_generated_files(portal_root: &Path) -> Result<Vec<PathBuf>, DynError> {
    let mut files = Vec::<PathBuf>::new();
    let index = portal_root.join("index.md");
    if index.is_file() {
        files.push(index);
    }
    let search = portal_root.join("search-index.json");
    if search.is_file() {
        files.push(search);
    }

    let lint_dir = portal_root.join("lints");
    if lint_dir.is_dir() {
        for entry in fs::read_dir(lint_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

fn render_index(lints: &[&LintSpec]) -> String {
    let mut out = String::new();
    out.push_str("# Lint Portal\n\n");
    out.push_str("Generated from canonical lint metadata.\n\n");

    out.push_str("## By Category\n\n");
    let mut by_category = BTreeMap::<&str, Vec<&LintSpec>>::new();
    for lint in lints {
        by_category
            .entry(lint.category.as_str())
            .or_default()
            .push(*lint);
    }
    for (category, items) in by_category {
        out.push_str(&format!("### {}\n\n", title_case(category)));
        for lint in items {
            out.push_str(&format!(
                "- [{}](lints/{}.md) (`{}` / `{}` / `{}`)\n",
                lint.id,
                lint.id.to_ascii_lowercase(),
                lint.pack,
                lint.maturity.as_str(),
                lint.policy
            ));
        }
        out.push('\n');
    }

    out.push_str("## By Maturity\n\n");
    let mut by_maturity = BTreeMap::<&str, Vec<&LintSpec>>::new();
    for lint in lints {
        by_maturity
            .entry(lint.maturity.as_str())
            .or_default()
            .push(*lint);
    }
    for (maturity, items) in by_maturity {
        out.push_str(&format!("### {}\n\n", title_case(maturity)));
        for lint in items {
            out.push_str(&format!("- `{}` ({})\n", lint.id, lint.pack));
        }
        out.push('\n');
    }

    out.push_str("## By Pack\n\n");
    let mut by_pack = BTreeMap::<&str, Vec<&LintSpec>>::new();
    for lint in lints {
        by_pack.entry(lint.pack).or_default().push(*lint);
    }
    for (pack, items) in by_pack {
        out.push_str(&format!("### {}\n\n", title_case(pack)));
        for lint in items {
            out.push_str(&format!(
                "- `{}` ({}, {})\n",
                lint.id,
                lint.category.as_str(),
                lint.maturity.as_str()
            ));
        }
        out.push('\n');
    }

    out
}

fn render_lint_page(lint: &LintSpec) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", lint.id));
    out.push_str(&format!("- Pack: `{}`\n", lint.pack));
    out.push_str(&format!("- Category: `{}`\n", lint.category.as_str()));
    out.push_str(&format!("- Maturity: `{}`\n", lint.maturity.as_str()));
    out.push_str(&format!("- Policy: `{}`\n", lint.policy));
    out.push_str(&format!("- Default Level: `{}`\n", lint.default_level));
    out.push_str(&format!(
        "- Confidence: `{}`\n",
        confidence_label(lint.confidence)
    ));
    out.push_str(&format!("- Introduced In: `{}`\n", lint.introduced_in));
    out.push_str(&format!(
        "- Lifecycle: `{}`\n\n",
        lifecycle_label(lint.lifecycle)
    ));

    out.push_str("## Summary\n\n");
    out.push_str(lint.docs.summary);
    out.push_str("\n\n## What It Does\n\n");
    out.push_str(lint.docs.what_it_does);
    out.push_str("\n\n## Why This Matters\n\n");
    out.push_str(lint.docs.why_this_matters);
    out.push_str("\n\n## Known Limitations\n\n");
    out.push_str(lint.docs.known_limitations);
    out.push_str("\n\n## How To Fix\n\n");
    out.push_str(lint.docs.how_to_fix);
    out.push_str("\n\n## Examples\n\n");
    for example in lint.docs.examples {
        out.push_str(&format!("- {}\n", example));
    }
    out.push_str("\n## References\n\n");
    for reference in lint.docs.references {
        out.push_str(&format!("- `{}`\n", reference));
    }

    out
}

fn render_search_index(lints: &[&LintSpec]) -> String {
    let entries = lints
        .iter()
        .map(|lint| {
            json!({
                "id": lint.id,
                "pack": lint.pack,
                "category": lint.category.as_str(),
                "maturity": lint.maturity.as_str(),
                "policy": lint.policy,
                "summary": lint.docs.summary,
                "path": format!("lints/{}.md", lint.id.to_ascii_lowercase()),
            })
        })
        .collect::<Vec<_>>();

    serde_json::to_string_pretty(&entries).expect("search index serialization should succeed")
}

fn confidence_label(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::Low => "low",
        Confidence::Medium => "medium",
        Confidence::High => "high",
    }
}

fn lifecycle_label(lifecycle: LintLifecycleState) -> String {
    match lifecycle {
        LintLifecycleState::Active => "active".to_string(),
        LintLifecycleState::Deprecated { since, .. } => format!("deprecated since {since}"),
        LintLifecycleState::Renamed { since, .. } => format!("renamed since {since}"),
        LintLifecycleState::Removed { since, .. } => format!("removed since {since}"),
    }
}

fn title_case(raw: &str) -> String {
    raw.split('_')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}
