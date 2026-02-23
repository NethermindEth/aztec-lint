use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub type DynError = Box<dyn Error + Send + Sync + 'static>;

pub fn workspace_root() -> Result<PathBuf, DynError> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or("failed to determine workspace root")?;
    Ok(root.to_path_buf())
}

pub fn parse_flags_and_options(
    args: &[String],
) -> Result<(BTreeSet<String>, BTreeMap<String, String>), DynError> {
    let mut flags = BTreeSet::new();
    let mut options = BTreeMap::new();
    let mut i = 0usize;
    while i < args.len() {
        let token = &args[i];
        if !token.starts_with("--") {
            return Err(format!("unexpected positional argument '{token}'").into());
        }
        if token == "--" {
            return Err("unexpected '--' separator".into());
        }

        if i + 1 < args.len() && !args[i + 1].starts_with("--") {
            let key = token.trim_start_matches("--").to_string();
            let value = args[i + 1].clone();
            if options.insert(key.clone(), value).is_some() {
                return Err(format!("duplicate option '--{key}'").into());
            }
            i += 2;
            continue;
        }

        let key = token.trim_start_matches("--").to_string();
        if !flags.insert(key.clone()) {
            return Err(format!("duplicate flag '--{key}'").into());
        }
        i += 1;
    }
    Ok((flags, options))
}

pub fn required_option(
    options: &mut BTreeMap<String, String>,
    key: &str,
) -> Result<String, DynError> {
    options
        .remove(key)
        .ok_or_else(|| format!("missing required option '--{key}'").into())
}

pub fn optional_option(options: &mut BTreeMap<String, String>, key: &str) -> Option<String> {
    options.remove(key)
}

pub fn ensure_no_unknown_options(
    flags: &BTreeSet<String>,
    options: &BTreeMap<String, String>,
) -> Result<(), DynError> {
    if !flags.is_empty() {
        let unknown = flags.iter().cloned().collect::<Vec<_>>().join(", ");
        return Err(format!("unknown flag(s): {unknown}").into());
    }
    if !options.is_empty() {
        let unknown = options.keys().cloned().collect::<Vec<_>>().join(", ");
        return Err(format!("unknown option(s): {unknown}").into());
    }
    Ok(())
}

pub fn write_text_file(path: &Path, contents: &str) -> Result<(), DynError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

pub fn read_text_file(path: &Path) -> Result<String, DynError> {
    Ok(fs::read_to_string(path)?)
}

pub fn render_template(mut template: String, replacements: &[(&str, String)]) -> String {
    for (key, value) in replacements {
        let needle = format!("{{{{{key}}}}}");
        template = template.replace(&needle, value);
    }
    template
}

pub fn run_command(command: &mut Command) -> Result<(), DynError> {
    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }

    let program = command.get_program().to_string_lossy().into_owned();
    let args = command
        .get_args()
        .map(OsStr::to_string_lossy)
        .map(|value| value.into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!("command failed: {program} {args}\n{stderr}").into())
}

pub fn normalize_rule_id(rule_id: &str) -> String {
    rule_id.trim().to_ascii_uppercase()
}

pub fn validate_rule_id(rule_id: &str) -> Result<(), DynError> {
    if rule_id.is_empty() {
        return Err("rule id cannot be empty".into());
    }
    if !rule_id
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        return Err(format!("rule id '{rule_id}' contains unsupported characters").into());
    }
    Ok(())
}
