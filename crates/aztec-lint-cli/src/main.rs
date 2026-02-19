#![forbid(unsafe_code)]

mod cli;
mod commands;

fn main() -> std::process::ExitCode {
    cli::run()
}
