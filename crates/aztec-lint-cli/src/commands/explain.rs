use std::process::ExitCode;

use aztec_lint_core::lints::LintLifecycleState;
use clap::Args;

use crate::cli::CliError;
use crate::commands::catalog::{confidence_label, find_rule};
use crate::exit_codes;

#[derive(Debug, Args)]
pub struct ExplainArgs {
    pub rule_id: String,
}

pub fn run(args: ExplainArgs) -> Result<ExitCode, CliError> {
    let rule_id = args.rule_id.trim().to_ascii_uppercase();
    let rule = find_rule(&rule_id).ok_or(CliError::UnknownRule { rule_id })?;

    println!("Rule: {}", rule.id);
    println!("Pack: {}", rule.pack);
    println!("Category: {}", rule.category.as_str());
    println!("Maturity: {}", rule.maturity.as_str());
    println!("Policy: {}", rule.policy);
    println!("Default Level: {}", rule.default_level);
    println!("Confidence: {}", confidence_label(rule.confidence));
    println!("Introduced In: {}", rule.introduced_in);
    println!("Lifecycle: {}", lifecycle_label(rule.lifecycle));

    match rule.lifecycle {
        LintLifecycleState::Deprecated {
            replacement, note, ..
        } => {
            if let Some(replacement) = replacement {
                println!("Replacement: {replacement}");
            }
            println!("Lifecycle Note: {note}");
        }
        LintLifecycleState::Renamed { to, .. } => {
            println!("Replacement: {to}");
        }
        LintLifecycleState::Removed { note, .. } => {
            println!("Lifecycle Note: {note}");
        }
        LintLifecycleState::Active => {}
    }

    println!();
    println!("Summary:");
    println!("{}", rule.docs.summary);
    println!();
    println!("What It Does:");
    println!("{}", rule.docs.what_it_does);
    println!();
    println!("Why This Matters:");
    println!("{}", rule.docs.why_this_matters);
    println!();
    println!("Known Limitations:");
    println!("{}", rule.docs.known_limitations);
    println!();
    println!("How To Fix:");
    println!("{}", rule.docs.how_to_fix);
    println!();
    println!("Examples:");
    for example in rule.docs.examples {
        println!("- {example}");
    }
    println!();
    println!("References:");
    for reference in rule.docs.references {
        println!("- {reference}");
    }
    Ok(exit_codes::success())
}

fn lifecycle_label(lifecycle: LintLifecycleState) -> String {
    match lifecycle {
        LintLifecycleState::Active => "active".to_string(),
        LintLifecycleState::Deprecated { since, .. } => format!("deprecated since {since}"),
        LintLifecycleState::Renamed { since, .. } => format!("renamed since {since}"),
        LintLifecycleState::Removed { since, .. } => format!("removed since {since}"),
    }
}
