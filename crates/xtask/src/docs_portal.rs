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
use crate::lint_intake::{IntakeEntry, parse_intake_table, validate_entries};

const INTAKE_STATUSES: [&str; 4] = ["covered", "accepted", "deferred", "rejected"];

pub fn run(args: &[String]) -> Result<(), DynError> {
    let (mut flags, options) = parse_flags_and_options(args)?;
    let check = flags.remove("check");
    ensure_no_unknown_options(&flags, &options)?;

    let root = workspace_root()?;
    let portal_root = root.join("docs/portal");

    let generated = build_generated_files(&root, &portal_root)?;

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

fn build_generated_files(
    workspace_root: &Path,
    portal_root: &Path,
) -> Result<BTreeMap<PathBuf, String>, DynError> {
    let mut files = BTreeMap::<PathBuf, String>::new();
    let all = all_lints().iter().collect::<Vec<_>>();
    let intake_entries = load_intake_entries(workspace_root)?;

    files.insert(
        portal_root.join("index.md"),
        render_index(&all, &intake_entries),
    );
    files.insert(
        portal_root.join("search-index.json"),
        render_search_index(&all),
    );
    files.insert(
        portal_root.join("roadmap/index.md"),
        render_roadmap_index(&intake_entries),
    );
    for status in INTAKE_STATUSES {
        files.insert(
            portal_root.join("roadmap").join(format!("{status}.md")),
            render_roadmap_status_page(status, &intake_entries),
        );
    }

    for lint in all {
        let page = portal_root
            .join("lints")
            .join(format!("{}.md", lint.id.to_ascii_lowercase()));
        files.insert(page, render_lint_page(lint));
    }
    Ok(files)
}

fn load_intake_entries(workspace_root: &Path) -> Result<Vec<IntakeEntry>, DynError> {
    let source_path = workspace_root.join("docs/NEW_LINTS.md");
    let source_text = read_text_file(&source_path)?;
    let entries = parse_intake_table(&source_text)?;
    validate_entries(&entries)?;
    Ok(entries)
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

    let roadmap_dir = portal_root.join("roadmap");
    if roadmap_dir.is_dir() {
        for entry in fs::read_dir(roadmap_dir)? {
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

fn render_index(lints: &[&LintSpec], intake_entries: &[IntakeEntry]) -> String {
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
    for (category, mut items) in by_category {
        items.sort_unstable_by_key(|lint| lint.id);
        out.push_str(&format!("### {}\n\n", title_case(category)));
        for lint in items {
            out.push_str(&format!(
                "- [{}](lints/{}.md) (`{}` / `{}` / `{}` / `{}`)\n",
                lint.id,
                lint.id.to_ascii_lowercase(),
                lint.pack,
                lint.maturity.as_str(),
                lint.policy,
                lifecycle_label(lint.lifecycle)
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
    for (maturity, mut items) in by_maturity {
        items.sort_unstable_by_key(|lint| lint.id);
        out.push_str(&format!("### {}\n\n", title_case(maturity)));
        for lint in items {
            out.push_str(&format!(
                "- `{}` ({}, {})\n",
                lint.id,
                lint.pack,
                lifecycle_label(lint.lifecycle)
            ));
        }
        out.push('\n');
    }

    out.push_str("## By Pack\n\n");
    let mut by_pack = BTreeMap::<&str, Vec<&LintSpec>>::new();
    for lint in lints {
        by_pack.entry(lint.pack).or_default().push(*lint);
    }
    for (pack, mut items) in by_pack {
        items.sort_unstable_by_key(|lint| lint.id);
        out.push_str(&format!("### {}\n\n", title_case(pack)));
        for lint in items {
            out.push_str(&format!(
                "- `{}` ({}, {}, {})\n",
                lint.id,
                lint.category.as_str(),
                lint.maturity.as_str(),
                lifecycle_label(lint.lifecycle)
            ));
        }
        out.push('\n');
    }

    out.push_str("## Roadmap Intake Views\n\n");
    out.push_str("Generated from intake decisions in `docs/NEW_LINTS.md`.\n\n");
    let counts = intake_counts(intake_entries);
    for status in INTAKE_STATUSES {
        out.push_str(&format!(
            "- [{}](roadmap/{}.md): `{}` proposal(s)\n",
            title_case(status),
            status,
            counts.get(status).copied().unwrap_or(0usize)
        ));
    }
    out.push('\n');
    out.push_str("- [All intake statuses](roadmap/index.md)\n");

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
    out.push_str("\n\n## Config Knobs\n\n");
    for knob in config_knobs(lint) {
        out.push_str(&format!("- {knob}\n"));
    }
    out.push_str("\n## Fix Safety Notes\n\n");
    for note in fix_safety_notes(lint) {
        out.push_str(&format!("- {note}\n"));
    }
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

fn config_knobs(lint: &LintSpec) -> Vec<String> {
    let mut knobs = vec![
        format!(
            "Enable this lint via ruleset selector `profile.<name>.ruleset = [\"{}\"]`.",
            lint.pack
        ),
        format!(
            "Target this maturity in-pack via `profile.<name>.ruleset = [\"{}@{}\"]`.",
            lint.pack,
            lint.maturity.as_str()
        ),
        format!(
            "Target this maturity across packs via `profile.<name>.ruleset = [\"tier:{}\"]` (alias `maturity:{}`).",
            lint.maturity.as_str(),
            lint.maturity.as_str()
        ),
        format!(
            "Override this lint level in config with `profile.<name>.deny|warn|allow = [\"{}\"]`.",
            lint.id
        ),
        format!(
            "Override this lint level in CLI with `--deny {0}`, `--warn {0}`, or `--allow {0}`.",
            lint.id
        ),
    ];

    if lint.pack == "aztec_pack" {
        knobs.push(
            "Aztec semantic-name knobs that can affect detection: `[aztec].external_attribute`, `[aztec].external_kinds`, `[aztec].only_self_attribute`, `[aztec].initializer_attribute`, `[aztec].enqueue_fn`, `[aztec].nullifier_fns`, and `[aztec.domain_separation].*`.".to_string(),
        );
    }

    knobs
}

fn fix_safety_notes(lint: &LintSpec) -> Vec<String> {
    vec![
        format!(
            "`aztec-lint fix` applies only safe fixes for `{}` and skips edits marked as needing review.",
            lint.id
        ),
        "Suggestion applicability `machine-applicable` maps to safe fixes.".to_string(),
        "Suggestion applicability `maybe-incorrect`, `has-placeholders`, and `unspecified` maps to `needs_review` and is not auto-applied.".to_string(),
        "Run `aztec-lint fix --dry-run` to inspect candidate edits before writing files.".to_string(),
    ]
}

fn render_roadmap_index(entries: &[IntakeEntry]) -> String {
    let mut out = String::new();
    out.push_str("# Roadmap Intake Views\n\n");
    out.push_str("Generated from intake decisions in `docs/NEW_LINTS.md`.\n\n");

    let counts = intake_counts(entries);
    for status in INTAKE_STATUSES {
        out.push_str(&format!(
            "- [{}]({}.md): `{}` proposal(s)\n",
            title_case(status),
            status,
            counts.get(status).copied().unwrap_or(0usize)
        ));
    }

    out
}

fn render_roadmap_status_page(status: &str, entries: &[IntakeEntry]) -> String {
    let filtered = entries
        .iter()
        .filter(|entry| intake_status_key(&entry.status) == status)
        .collect::<Vec<_>>();

    let mut out = String::new();
    out.push_str(&format!("# Intake Status: {}\n\n", title_case(status)));
    out.push_str("Generated from intake decisions in `docs/NEW_LINTS.md`.\n\n");
    out.push_str(&format!("- Status: `{status}`\n"));
    out.push_str(&format!("- Proposal count: `{}`\n\n", filtered.len()));

    out.push_str("| Proposal | Canonical mapping | Notes |\n");
    out.push_str("|---|---|---|\n");
    for entry in filtered {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            markdown_table_escape(&entry.proposal),
            markdown_table_escape(&entry.canonical_mapping),
            markdown_table_escape(&entry.notes)
        ));
    }
    out.push('\n');
    out.push_str("[Back to intake index](index.md)\n");

    out
}

fn intake_counts(entries: &[IntakeEntry]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::<String, usize>::new();
    for entry in entries {
        let status = intake_status_key(&entry.status);
        *counts.entry(status).or_insert(0usize) += 1;
    }
    counts
}

fn intake_status_key(raw: &str) -> String {
    raw.trim().trim_matches('`').to_ascii_lowercase()
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
                "lifecycle": lifecycle_label(lint.lifecycle),
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

fn markdown_table_escape(raw: &str) -> String {
    raw.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('\n', "<br>")
}
