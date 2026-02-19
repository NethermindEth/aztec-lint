use std::process::ExitCode;

pub const SUCCESS: u8 = 0;
pub const DIAGNOSTICS_FOUND: u8 = 1;
pub const INTERNAL_ERROR: u8 = 2;

pub fn success() -> ExitCode {
    ExitCode::from(SUCCESS)
}

pub fn diagnostics_found(blocking_diagnostics: bool) -> ExitCode {
    if blocking_diagnostics {
        ExitCode::from(DIAGNOSTICS_FOUND)
    } else {
        success()
    }
}

pub fn internal_error() -> ExitCode {
    ExitCode::from(INTERNAL_ERROR)
}

pub fn clap_exit(code: i32) -> ExitCode {
    match u8::try_from(code) {
        Ok(value) => ExitCode::from(value),
        Err(_) => internal_error(),
    }
}
