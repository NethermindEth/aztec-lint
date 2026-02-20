#![forbid(unsafe_code)]

#[path = "../cli.rs"]
mod cli;
#[path = "../commands/mod.rs"]
mod commands;
#[path = "../exit_codes.rs"]
mod exit_codes;

fn main() -> std::process::ExitCode {
    cli::run()
}
