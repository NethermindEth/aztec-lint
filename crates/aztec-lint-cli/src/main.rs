#![forbid(unsafe_code)]

mod cli;
mod commands;
mod exit_codes;

fn main() -> std::process::ExitCode {
    cli::run()
}
