use std::error::Error;
use std::fmt::{Display, Formatter};
use std::process::ExitCode;

use aztec_lint_core::config::{ConfigError, RuleOverrides};
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

use crate::commands::{aztec_scan, check, explain, fix, rules};
use crate::exit_codes;

#[derive(Debug)]
pub enum CliError {
    Config(ConfigError),
    UnknownRule { rule_id: String },
    Runtime(String),
}

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(err) => write!(f, "{err}"),
            Self::UnknownRule { rule_id } => {
                write!(f, "unknown rule id '{rule_id}' (run `aztec-lint rules`)")
            }
            Self::Runtime(message) => write!(f, "{message}"),
        }
    }
}

impl Error for CliError {}

impl From<ConfigError> for CliError {
    fn from(value: ConfigError) -> Self {
        Self::Config(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Sarif,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum SeverityThreshold {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum MinConfidence {
    High,
    Medium,
    Low,
}

#[derive(Clone, Debug, Args)]
pub struct CommonLintFlags {
    #[arg(long, default_value = "text", value_enum)]
    pub format: OutputFormat,
    #[arg(long, default_value = "warning", value_enum)]
    pub severity_threshold: SeverityThreshold,
    #[arg(long = "deny", value_name = "RULE_ID", action = ArgAction::Append)]
    pub deny: Vec<String>,
    #[arg(long = "warn", value_name = "RULE_ID", action = ArgAction::Append)]
    pub warn: Vec<String>,
    #[arg(long = "allow", value_name = "RULE_ID", action = ArgAction::Append)]
    pub allow: Vec<String>,
    #[arg(long, default_value = "low", value_enum)]
    pub min_confidence: MinConfidence,
    #[arg(long)]
    pub show_suppressed: bool,
}

impl CommonLintFlags {
    pub fn rule_overrides(&self) -> RuleOverrides {
        RuleOverrides {
            deny: self.deny.clone(),
            warn: self.warn.clone(),
            allow: self.allow.clone(),
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "aztec-lint", version, about = "Aztec/Noir linting CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Check(check::CheckArgs),
    Fix(fix::FixArgs),
    Rules(rules::RulesArgs),
    Explain(explain::ExplainArgs),
    Aztec(AztecArgs),
}

#[derive(Debug, Args)]
struct AztecArgs {
    #[command(subcommand)]
    command: AztecSubcommand,
}

#[derive(Debug, Subcommand)]
enum AztecSubcommand {
    Scan(aztec_scan::AztecScanArgs),
}

pub fn run() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(value) => value,
        Err(err) => {
            let code = exit_codes::clap_exit(err.exit_code());
            let _ = err.print();
            return code;
        }
    };

    match dispatch(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            exit_codes::internal_error()
        }
    }
}

fn dispatch(cli: Cli) -> Result<ExitCode, CliError> {
    match cli.command {
        Command::Check(args) => check::run(args),
        Command::Fix(args) => fix::run(args),
        Command::Rules(args) => rules::run(args),
        Command::Explain(args) => explain::run(args),
        Command::Aztec(args) => match args.command {
            AztecSubcommand::Scan(scan_args) => aztec_scan::run(scan_args),
        },
    }
}
