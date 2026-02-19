use std::process::ExitCode;

use clap::Args;

use crate::cli::CliError;
use crate::commands::catalog::find_rule;

#[derive(Debug, Args)]
pub struct ExplainArgs {
    pub rule_id: String,
}

pub fn run(args: ExplainArgs) -> Result<ExitCode, CliError> {
    let rule_id = args.rule_id.trim().to_ascii_uppercase();
    let rule = find_rule(&rule_id).ok_or(CliError::UnknownRule { rule_id })?;
    println!("Rule: {}", rule.id);
    println!("Pack: {}", rule.pack);
    println!("Policy: {}", rule.policy);
    println!("Confidence: {}", rule.confidence);
    println!("Summary: {}", rule.summary);
    Ok(ExitCode::from(0))
}
