use std::process::ExitCode;

use clap::Args;

use crate::cli::CliError;
use crate::commands::catalog::{all_rules, confidence_label};
use crate::exit_codes;

#[derive(Debug, Args)]
pub struct RulesArgs {}

pub fn run(_args: RulesArgs) -> Result<ExitCode, CliError> {
    println!("RULE_ID\tPACK\tCATEGORY\tMATURITY\tPOLICY\tCONFIDENCE\tSUMMARY");
    for rule in all_rules() {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            rule.id,
            rule.pack,
            rule.category.as_str(),
            rule.maturity.as_str(),
            rule.policy,
            confidence_label(rule.confidence),
            rule.docs.summary
        );
    }
    Ok(exit_codes::success())
}
