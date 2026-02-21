use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Args;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;
use tempfile::tempdir;
use ureq::Error as UreqError;
use zip::ZipArchive;

use crate::cli::CliError;
use crate::exit_codes;

const DEFAULT_REPO: &str = "NethermindEth/aztec-lint";
const TOOL_NAME: &str = "aztec-lint";

#[derive(Debug, Args)]
pub struct UpdateArgs {
    #[arg(long, default_value = "latest", value_name = "VERSION")]
    pub version: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ReleaseSelector {
    Latest,
    Tag(String),
}

#[derive(Clone, Copy, Debug)]
struct TargetPlatform {
    os: &'static str,
    arch: &'static str,
    archive_ext: &'static str,
    binary_name: &'static str,
}

pub fn run(args: UpdateArgs) -> Result<ExitCode, CliError> {
    let selector = normalize_version_input(&args.version)?;
    let target = detect_target_platform()?;
    let repo = env::var("AZTEC_LINT_REPO").unwrap_or_else(|_| DEFAULT_REPO.to_string());

    let base_url = match &selector {
        ReleaseSelector::Latest => format!("https://github.com/{repo}/releases/latest/download"),
        ReleaseSelector::Tag(tag) => format!("https://github.com/{repo}/releases/download/{tag}"),
    };

    let asset_name = format!(
        "{TOOL_NAME}-{}-{}.{}",
        target.os, target.arch, target.archive_ext
    );
    let checksums_name = "checksums.txt";
    let asset_url = format!("{base_url}/{asset_name}");
    let checksums_url = format!("{base_url}/{checksums_name}");

    let tmp = tempdir().map_err(|source| {
        CliError::Runtime(format!("failed to create temp directory: {source}"))
    })?;
    let asset_path = tmp.path().join(&asset_name);
    let checksums_path = tmp.path().join(checksums_name);
    let extracted_binary_path = tmp.path().join(target.binary_name);

    println!("Downloading {asset_name}...");
    download_to_file(&asset_url, &asset_path)?;

    println!("Downloading {checksums_name}...");
    download_to_file(&checksums_url, &checksums_path)?;

    let checksums = fs::read_to_string(&checksums_path).map_err(|source| {
        CliError::Runtime(format!(
            "failed to read downloaded checksums '{}': {source}",
            checksums_path.display()
        ))
    })?;

    let expected = expected_checksum_for_asset(&checksums, &asset_name).ok_or_else(|| {
        CliError::Runtime(format!(
            "checksums file does not contain an entry for '{asset_name}'"
        ))
    })?;

    let actual = sha256_file(&asset_path)?;
    if !actual.eq_ignore_ascii_case(&expected) {
        return Err(CliError::Runtime(format!(
            "checksum mismatch for '{asset_name}': expected {expected}, got {actual}"
        )));
    }

    extract_binary(
        &asset_path,
        target.binary_name,
        target.archive_ext,
        &extracted_binary_path,
    )?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(&extracted_binary_path, fs::Permissions::from_mode(0o755)).map_err(
            |source| {
                CliError::Runtime(format!(
                    "failed to mark extracted binary executable '{}': {source}",
                    extracted_binary_path.display()
                ))
            },
        )?;
    }

    self_replace::self_replace(&extracted_binary_path).map_err(|source| {
        CliError::Runtime(format!(
            "failed to replace current executable with '{}': {source}",
            extracted_binary_path.display()
        ))
    })?;

    match selector {
        ReleaseSelector::Latest => {
            println!("Updated aztec-lint to the latest available release.");
        }
        ReleaseSelector::Tag(tag) => {
            println!("Updated aztec-lint to {tag}.");
        }
    }
    println!("Run `aztec-lint --version` to verify the installed version.");

    Ok(exit_codes::success())
}

fn normalize_version_input(input: &str) -> Result<ReleaseSelector, CliError> {
    let trimmed = input.trim();
    if trimmed.eq_ignore_ascii_case("latest") {
        return Ok(ReleaseSelector::Latest);
    }

    if is_semver_core(trimmed) {
        return Ok(ReleaseSelector::Tag(format!("v{trimmed}")));
    }

    if let Some(rest) = trimmed.strip_prefix('v')
        && is_semver_core(rest)
    {
        return Ok(ReleaseSelector::Tag(trimmed.to_string()));
    }

    Err(CliError::Runtime(format!(
        "invalid version '{trimmed}'; expected 'latest', 'vX.Y.Z', or 'X.Y.Z'"
    )))
}

fn is_semver_core(input: &str) -> bool {
    let parts = input.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

fn detect_target_platform() -> Result<TargetPlatform, CliError> {
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => Ok(TargetPlatform {
            os: "linux",
            arch: "x86_64",
            archive_ext: "tar.gz",
            binary_name: TOOL_NAME,
        }),
        ("macos", "x86_64") => Ok(TargetPlatform {
            os: "macos",
            arch: "x86_64",
            archive_ext: "tar.gz",
            binary_name: TOOL_NAME,
        }),
        ("macos", "aarch64") => Ok(TargetPlatform {
            os: "macos",
            arch: "aarch64",
            archive_ext: "tar.gz",
            binary_name: TOOL_NAME,
        }),
        ("windows", "x86_64") => Ok(TargetPlatform {
            os: "windows",
            arch: "x86_64",
            archive_ext: "zip",
            binary_name: "aztec-lint.exe",
        }),
        (os, arch) => Err(CliError::Runtime(format!(
            "unsupported platform for update: {os}-{arch}"
        ))),
    }
}

fn download_to_file(url: &str, path: &Path) -> Result<(), CliError> {
    let response = ureq::get(url).call().map_err(|source| match source {
        UreqError::Status(code, response) => CliError::Runtime(format!(
            "download failed with HTTP {code} for '{url}' ({})",
            response.status_text()
        )),
        UreqError::Transport(transport) => {
            CliError::Runtime(format!("download failed for '{url}': {transport}"))
        }
    })?;

    let mut reader = response.into_reader();
    let mut out = File::create(path).map_err(|source| {
        CliError::Runtime(format!(
            "failed to create download file '{}': {source}",
            path.display()
        ))
    })?;

    std::io::copy(&mut reader, &mut out).map_err(|source| {
        CliError::Runtime(format!(
            "failed to write download file '{}': {source}",
            path.display()
        ))
    })?;

    out.flush().map_err(|source| {
        CliError::Runtime(format!(
            "failed to flush download file '{}': {source}",
            path.display()
        ))
    })
}

fn expected_checksum_for_asset(checksums: &str, asset_name: &str) -> Option<String> {
    checksums.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Some((hash, name)) = trimmed.split_once("  ")
            && name.trim() == asset_name
        {
            return Some(hash.trim().to_string());
        }

        let mut parts = trimmed.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?;
        if parts.next().is_none() && name == asset_name {
            return Some(hash.to_string());
        }
        None
    })
}

fn sha256_file(path: &Path) -> Result<String, CliError> {
    let mut file = File::open(path).map_err(|source| {
        CliError::Runtime(format!(
            "failed to open file for checksum '{}': {source}",
            path.display()
        ))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = file.read(&mut buffer).map_err(|source| {
            CliError::Runtime(format!(
                "failed to read file for checksum '{}': {source}",
                path.display()
            ))
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn extract_binary(
    archive_path: &Path,
    binary_name: &str,
    archive_ext: &str,
    destination: &Path,
) -> Result<PathBuf, CliError> {
    match archive_ext {
        "tar.gz" => extract_binary_from_tar_gz(archive_path, binary_name, destination),
        "zip" => extract_binary_from_zip(archive_path, binary_name, destination),
        unsupported => Err(CliError::Runtime(format!(
            "unsupported archive extension '{unsupported}'"
        ))),
    }
}

fn extract_binary_from_tar_gz(
    archive_path: &Path,
    binary_name: &str,
    destination: &Path,
) -> Result<PathBuf, CliError> {
    let file = File::open(archive_path).map_err(|source| {
        CliError::Runtime(format!(
            "failed to open archive '{}': {source}",
            archive_path.display()
        ))
    })?;
    let gz = GzDecoder::new(file);
    let mut archive = Archive::new(gz);

    let entries = archive.entries().map_err(|source| {
        CliError::Runtime(format!(
            "failed to read archive entries '{}': {source}",
            archive_path.display()
        ))
    })?;

    for entry in entries {
        let mut entry = entry.map_err(|source| {
            CliError::Runtime(format!(
                "failed to read archive entry '{}': {source}",
                archive_path.display()
            ))
        })?;

        let path = entry.path().map_err(|source| {
            CliError::Runtime(format!(
                "failed to inspect archive entry path '{}': {source}",
                archive_path.display()
            ))
        })?;

        if path.file_name().is_some_and(|name| name == binary_name) {
            let mut out = File::create(destination).map_err(|source| {
                CliError::Runtime(format!(
                    "failed to create extracted binary '{}': {source}",
                    destination.display()
                ))
            })?;
            std::io::copy(&mut entry, &mut out).map_err(|source| {
                CliError::Runtime(format!(
                    "failed to extract binary '{}' from '{}': {source}",
                    binary_name,
                    archive_path.display()
                ))
            })?;
            out.flush().map_err(|source| {
                CliError::Runtime(format!(
                    "failed to flush extracted binary '{}': {source}",
                    destination.display()
                ))
            })?;
            return Ok(destination.to_path_buf());
        }
    }

    Err(CliError::Runtime(format!(
        "binary '{binary_name}' not found in archive '{}'",
        archive_path.display()
    )))
}

fn extract_binary_from_zip(
    archive_path: &Path,
    binary_name: &str,
    destination: &Path,
) -> Result<PathBuf, CliError> {
    let file = File::open(archive_path).map_err(|source| {
        CliError::Runtime(format!(
            "failed to open archive '{}': {source}",
            archive_path.display()
        ))
    })?;
    let mut archive = ZipArchive::new(file).map_err(|source| {
        CliError::Runtime(format!(
            "failed to read zip archive '{}': {source}",
            archive_path.display()
        ))
    })?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|source| {
            CliError::Runtime(format!(
                "failed to read zip entry {} from '{}': {source}",
                index,
                archive_path.display()
            ))
        })?;

        if entry.is_dir() {
            continue;
        }

        let entry_path = Path::new(entry.name());
        if entry_path
            .file_name()
            .is_some_and(|name| name == binary_name)
        {
            let mut out = File::create(destination).map_err(|source| {
                CliError::Runtime(format!(
                    "failed to create extracted binary '{}': {source}",
                    destination.display()
                ))
            })?;
            std::io::copy(&mut entry, &mut out).map_err(|source| {
                CliError::Runtime(format!(
                    "failed to extract binary '{}' from '{}': {source}",
                    binary_name,
                    archive_path.display()
                ))
            })?;
            out.flush().map_err(|source| {
                CliError::Runtime(format!(
                    "failed to flush extracted binary '{}': {source}",
                    destination.display()
                ))
            })?;
            return Ok(destination.to_path_buf());
        }
    }

    Err(CliError::Runtime(format!(
        "binary '{binary_name}' not found in archive '{}'",
        archive_path.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::{ReleaseSelector, expected_checksum_for_asset, normalize_version_input};

    #[test]
    fn normalize_version_accepts_latest() {
        assert_eq!(
            normalize_version_input("latest").expect("latest should parse"),
            ReleaseSelector::Latest
        );
    }

    #[test]
    fn normalize_version_accepts_prefixed_tag() {
        assert_eq!(
            normalize_version_input("v1.2.3").expect("prefixed tag should parse"),
            ReleaseSelector::Tag("v1.2.3".to_string())
        );
    }

    #[test]
    fn normalize_version_accepts_unprefixed_tag() {
        assert_eq!(
            normalize_version_input("1.2.3").expect("semver should parse"),
            ReleaseSelector::Tag("v1.2.3".to_string())
        );
    }

    #[test]
    fn normalize_version_rejects_invalid_input() {
        let err = normalize_version_input("v1.2")
            .expect_err("invalid version should fail")
            .to_string();
        assert!(err.contains("invalid version"));
    }

    #[test]
    fn checksum_lookup_accepts_double_space_format() {
        let checksums = "abc123  aztec-lint-linux-x86_64.tar.gz\ndef456  other.tar.gz\n";
        assert_eq!(
            expected_checksum_for_asset(checksums, "aztec-lint-linux-x86_64.tar.gz"),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn checksum_lookup_accepts_whitespace_format() {
        let checksums = "abc123 aztec-lint-windows-x86_64.zip\n";
        assert_eq!(
            expected_checksum_for_asset(checksums, "aztec-lint-windows-x86_64.zip"),
            Some("abc123".to_string())
        );
    }
}
