use std::collections::BTreeMap;
use std::path::PathBuf;

use aztec_lint_core::lints::{LintMaturityTier, find_lint};
use aztec_lint_core::policy::is_supported_policy;

use crate::common::{
    DynError, ensure_no_unknown_options, normalize_rule_id, optional_option,
    parse_flags_and_options, read_text_file, render_template, required_option, validate_rule_id,
    workspace_root, write_text_file,
};

pub fn run(args: &[String]) -> Result<(), DynError> {
    let (mut flags, mut options) = parse_flags_and_options(args)?;
    let dry_run = flags.remove("dry-run");

    let rule_id = normalize_rule_id(&required_option(&mut options, "id")?);
    validate_rule_id(&rule_id)?;
    let pack = required_option(&mut options, "pack")?;
    let category = required_option(&mut options, "category")?.to_ascii_lowercase();
    let tier_raw = required_option(&mut options, "tier")?;
    let tier = LintMaturityTier::parse(&tier_raw).ok_or_else(|| {
        format!("invalid tier '{tier_raw}'; expected stable|preview|experimental")
    })?;

    if find_lint(&rule_id).is_some() {
        return Err(format!("lint id '{rule_id}' already exists in canonical catalog").into());
    }

    if !matches!(
        category.as_str(),
        "correctness" | "maintainability" | "privacy" | "protocol" | "soundness"
    ) {
        return Err(format!(
            "invalid category '{category}'; expected correctness|maintainability|privacy|protocol|soundness"
        )
        .into());
    }

    let mut policy = optional_option(&mut options, "policy")
        .unwrap_or_else(|| category.clone())
        .trim()
        .to_ascii_lowercase();
    if policy == "cost" {
        policy = "performance".to_string();
    }
    if !is_supported_policy(&policy) {
        return Err(format!(
            "invalid policy '{policy}'; expected one of privacy|protocol|soundness|correctness|maintainability|performance"
        )
        .into());
    }
    ensure_no_unknown_options(&flags, &options)?;

    let root = workspace_root()?;
    let rule_source_dir = root.join(pack_source_dir(&pack)?);
    let fixture_dir = root.join(pack_fixture_dir(&pack)?);

    let module_name = rule_id.to_ascii_lowercase();
    let struct_name = to_rust_type_name(&rule_id);
    let rule_file = rule_source_dir.join(format!("{module_name}.rs"));
    let fixture_positive = fixture_dir.join(format!("{}_positive.nr", module_name));
    let fixture_negative = fixture_dir.join(format!("{}_negative.nr", module_name));
    let fixture_suppressed = fixture_dir.join(format!("{}_suppressed.nr", module_name));
    let test_stub = root
        .join("crates/aztec-lint-rules/tests/generated")
        .join(format!("{module_name}.rs"));
    let metadata_snippet = root
        .join("crates/aztec-lint-core/src/lints/scaffold")
        .join(format!("{rule_id}.lint_spec.snippet"));
    let registry_snippet = root
        .join("crates/aztec-lint-rules/src/engine/scaffold")
        .join(format!("{rule_id}.registry.snippet"));

    let template_dir = root.join("crates/xtask/templates");
    let rule_template = read_text_file(&template_dir.join("rule.rs.tmpl"))?;
    let fixture_positive_template =
        read_text_file(&template_dir.join("rule_fixture_positive.nr.tmpl"))?;
    let fixture_negative_template =
        read_text_file(&template_dir.join("rule_fixture_negative.nr.tmpl"))?;
    let fixture_suppressed_template =
        read_text_file(&template_dir.join("rule_fixture_suppressed.nr.tmpl"))?;

    let replacements = vec![
        ("rule_id", rule_id.clone()),
        ("module_name", module_name.clone()),
        ("struct_name", struct_name.clone()),
        ("pack", pack.clone()),
        ("category", category.clone()),
        ("tier", tier.as_str().to_string()),
        ("policy", policy.clone()),
    ];

    let rule_source = render_template(rule_template, &replacements);
    let positive_source = render_template(fixture_positive_template, &replacements);
    let negative_source = render_template(fixture_negative_template, &replacements);
    let suppressed_source = render_template(fixture_suppressed_template, &replacements);

    let metadata_source = render_metadata_snippet(&rule_id, &pack, &category, tier, &policy);
    let registry_source = render_registry_snippet(&pack, &module_name, &struct_name);
    let test_source = render_test_stub(&rule_id, &module_name);

    let mut planned = BTreeMap::<PathBuf, String>::new();
    planned.insert(rule_file, rule_source);
    planned.insert(fixture_positive, positive_source);
    planned.insert(fixture_negative, negative_source);
    planned.insert(fixture_suppressed, suppressed_source);
    planned.insert(metadata_snippet, metadata_source);
    planned.insert(registry_snippet, registry_source);
    planned.insert(test_stub, test_source);

    if dry_run {
        println!("new-lint dry-run for {rule_id}");
        for path in planned.keys() {
            println!("  create {}", path.display());
        }
        return Ok(());
    }

    ensure_paths_do_not_exist(planned.keys())?;
    for (path, contents) in planned {
        write_text_file(&path, &contents)?;
    }

    println!("scaffolded lint {rule_id}");
    println!(
        "next: wire snippets into canonical catalog and runtime registry, then run `cargo xtask update-lints --check`"
    );
    Ok(())
}

fn ensure_paths_do_not_exist<'a>(paths: impl Iterator<Item = &'a PathBuf>) -> Result<(), DynError> {
    let mut existing = Vec::<String>::new();
    for path in paths {
        if path.exists() {
            existing.push(path.display().to_string());
        }
    }
    if existing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "refusing to overwrite existing scaffold files: {}",
        existing.join(", ")
    )
    .into())
}

fn pack_source_dir(pack: &str) -> Result<&'static str, DynError> {
    match pack {
        "noir_core" => Ok("crates/aztec-lint-rules/src/noir_core"),
        "aztec_pack" => Ok("crates/aztec-lint-rules/src/aztec"),
        other => Err(format!("unsupported pack '{other}'").into()),
    }
}

fn pack_fixture_dir(pack: &str) -> Result<&'static str, DynError> {
    match pack {
        "noir_core" => Ok("fixtures/noir_core/rule_cases"),
        "aztec_pack" => Ok("fixtures/aztec/rule_cases"),
        other => Err(format!("unsupported pack '{other}'").into()),
    }
}

fn to_rust_type_name(rule_id: &str) -> String {
    let mut out = String::new();
    for segment in rule_id.split('_') {
        if segment.is_empty() {
            continue;
        }
        let mut chars = segment.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.push_str(chars.as_str().to_ascii_lowercase().as_str());
        }
    }
    out.push_str("Rule");
    out
}

fn render_metadata_snippet(
    rule_id: &str,
    pack: &str,
    category: &str,
    tier: LintMaturityTier,
    policy: &str,
) -> String {
    format!(
        "// Insert into ALL_LINT_SPECS in crates/aztec-lint-core/src/lints/mod.rs\n\
LintSpec {{\n\
    id: \"{rule_id}\",\n\
    pack: \"{pack}\",\n\
    policy: \"{policy}\",\n\
    category: LintCategory::{},\n\
    maturity: LintMaturityTier::{},\n\
    introduced_in: INTRODUCED_IN_V0_1_0,\n\
    default_level: RuleLevel::Warn,\n\
    confidence: Confidence::Medium,\n\
    lifecycle: LintLifecycleState::Active,\n\
    docs: LintDocs {{\n\
        summary: \"TODO\",\n\
        what_it_does: \"TODO\",\n\
        why_this_matters: \"TODO\",\n\
        known_limitations: \"TODO\",\n\
        how_to_fix: \"TODO\",\n\
        examples: &[\"TODO\"],\n\
        references: &[DOCS_REFERENCE_RULE_AUTHORING],\n\
    }},\n\
}},\n",
        to_lint_category_variant(category),
        to_tier_variant(tier),
    )
}

fn render_registry_snippet(pack: &str, module_name: &str, struct_name: &str) -> String {
    let import_prefix = match pack {
        "noir_core" => "crate::noir_core",
        "aztec_pack" => "crate::aztec",
        _ => "crate",
    };

    format!(
        "// Add import in crates/aztec-lint-rules/src/engine/registry.rs\n\
use {import_prefix}::{module_name}::{struct_name};\n\n\
// Add registration in full_registry()\n\
register(Box::new({struct_name})),\n"
    )
}

fn render_test_stub(rule_id: &str, module_name: &str) -> String {
    format!(
        "// Baseline test stub for {rule_id}.\n\
// Move/merge into existing pack-specific integration tests when implementing the rule.\n\
\n#[test]\nfn {module_name}_baseline_stub() {{\n    // TODO: add positive/negative/suppressed assertions for {rule_id}.\n    assert!(true);\n}}\n"
    )
}

fn to_lint_category_variant(category: &str) -> &'static str {
    match category {
        "correctness" => "Correctness",
        "maintainability" => "Maintainability",
        "privacy" => "Privacy",
        "protocol" => "Protocol",
        "soundness" => "Soundness",
        _ => "Correctness",
    }
}

fn to_tier_variant(tier: LintMaturityTier) -> &'static str {
    match tier {
        LintMaturityTier::Stable => "Stable",
        LintMaturityTier::Preview => "Preview",
        LintMaturityTier::Experimental => "Experimental",
    }
}
