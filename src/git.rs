use crate::errors::*;

use crate::config::{CommitRange, Config, FixupMode};
use regex::Regex;
use semver::{Version, VersionReq};
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub struct GitConfigBuilder {
    key: &'static str,
    default: Option<&'static str>,
    value_type: Option<&'static str>,
}

impl GitConfigBuilder {
    pub const fn new(key: &'static str) -> Self {
        Self {
            key,
            default: None,
            value_type: None,
        }
    }

    pub const fn with_default(mut self, default: &'static str) -> Self {
        self.default = Some(default);
        self
    }

    pub const fn with_type(mut self, value_type: &'static str) -> Self {
        self.value_type = Some(value_type);
        self
    }

    pub fn get(&self) -> Result<Option<String>> {
        let mut args = vec!["config", "--get"];
        if let Some(default) = self.default {
            args.push("--default");
            args.push(default);
        }
        if let Some(value_type) = self.value_type {
            args.push("--type");
            args.push(value_type);
        }
        args.push(self.key);

        let output = Command::new("git")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args(&args)
            .output()?;
        if !output.status.success() {
            match output.status.code() {
                Some(1) => {
                    return Ok(None);
                }
                _ => bail!("{}", String::from_utf8_lossy(&output.stderr).trim()),
            }
        }
        Ok(Some(
            String::from_utf8_lossy(&output.stdout)
                .trim_end()
                .to_owned(),
        ))
    }

    pub fn get_as_bool(&self) -> Result<Option<bool>> {
        let value = self.get()?;
        match value {
            None => Ok(None),
            Some(value) => {
                Ok(Some(value.parse::<bool>().with_context(|| {
                    anyhow!("Failed to parse key '{}' as bool", self.key)
                })?))
            }
        }
    }

    pub fn get_as_int(&self) -> Result<Option<u32>> {
        let value = self.get()?;
        match value {
            None => Ok(None),
            Some(value) => {
                Ok(Some(value.parse::<u32>().with_context(|| {
                    anyhow!("Failed to parse key '{}' as u32", self.key)
                })?))
            }
        }
    }
}

pub fn git_rebase(rev: &str, interactive: bool) -> Result<()> {
    let root = git_rev_root().context("failed to get git rev root")?;
    let rev = match root.starts_with(rev) {
        true => "--root".to_string(),
        false => format!("{}^", rev),
    };

    let args = vec![
        "rebase",
        "--interactive",
        "--autosquash",
        "--autostash",
        &rev,
    ];
    let mut cmd = Command::new("git");
    if !interactive {
        cmd.env("GIT_EDITOR", "true");
        cmd.env("GIT_SEQUENCE_EDITOR", "true");
    }
    let cmd = cmd.args(&args).spawn()?;
    let output = cmd.wait_with_output()?;

    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim_end());
    }

    Ok(())
}

pub fn git_rev_root() -> Result<String> {
    let args = vec!["rev-list", "--max-parents=0", "--no-abbrev-commit", "HEAD"];
    let output = Command::new("git")
        .stdout(Stdio::piped())
        .args(&args)
        .output()?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim_end());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .into_owned()
        .trim_end()
        .to_owned())
}

pub fn git_rev_range(config: &Config) -> Result<Option<String>> {
    let head = "HEAD".to_string();

    match &config.range {
        CommitRange::All => Ok(Some(head)),
        CommitRange::Local => {
            let upstream = git_rev_parse("@{upstream}");
            if let Ok(upstream) = upstream {
                let head = git_rev_parse("HEAD").context("failed to rev parse HEAD")?;
                if upstream == head {
                    return Ok(None);
                }
                return Ok(Some("@{upstream}..HEAD".to_string()));
            }
            Ok(Some(head))
        }
        CommitRange::Range(range) => Ok(Some(range.into())),
    }
}

pub fn git_rev_parse(rev: &str) -> Result<String> {
    let args = vec!["rev-parse", rev];
    let output = Command::new("git")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(&args)
        .output()?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim_end());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .into_owned()
        .trim_end()
        .to_owned())
}

pub fn git_rev_list(rev: &str, max_count: u32) -> Result<Vec<String>> {
    let max_count = format!("{}", max_count);
    let args = vec!["rev-list", "-n", &max_count, "--no-abbrev-commit", rev];
    let output = Command::new("git")
        .stdout(Stdio::piped())
        .args(&args)
        .output()?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim_end());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .into_owned()
        .trim_end()
        .to_owned()
        .lines()
        .map(|e| e.to_owned())
        .collect())
}

pub fn git_toplevel() -> Result<PathBuf> {
    git_rev_parse("--show-toplevel").map(PathBuf::from)
}

pub fn is_valid_git_rev(rev: &str) -> Result<bool> {
    let files_args = vec!["rev-parse", "--verify", rev];
    let mut cmd = Command::new("git")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(&files_args)
        .spawn()?;
    Ok(cmd.wait()?.success())
}

pub fn git_commit_fixup(target: &str, mode: FixupMode) -> Result<()> {
    let fixup = mode.to_cli_option(target);
    let mut args = vec!["commit", "--verbose", &fixup];
    if matches!(mode, FixupMode::Fixup) {
        args.push("--no-edit")
    };
    let output = Command::new("git")
        .args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim_end());
    }
    Ok(())
}

pub fn git_staged_files() -> Result<Vec<String>> {
    let files_args = vec![
        "--no-pager",
        "diff",
        "--color=never",
        "--name-only",
        "--cached",
        "--no-ext-diff",
    ];
    let output = Command::new("git")
        .stdout(Stdio::piped())
        .args(&files_args)
        .output()?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim_end());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .lines()
        .map(|e| e.to_owned())
        .collect())
}

pub fn git_version() -> Result<Version> {
    let args = vec!["version"];
    let output = Command::new("git")
        .stdout(Stdio::piped())
        .args(&args)
        .output()?;
    if !output.status.success() {
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim_end());
    }
    let git_version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let version_regex = Regex::new(r"[^ ]+ [^ ]+ (?P<version>[^ ]+)")
        .context("failed to create git version regex")?;
    let captures = version_regex
        .captures(&git_version)
        .with_context(|| format!("Failed to match git version from '{}'", git_version))?;
    let version = captures
        .name("version")
        .context("failed to get version capture group")?
        .as_str();
    let version = Version::parse(version)
        .with_context(|| format!("failed to parse version from '{}'", version))?;

    Ok(version)
}

pub fn git_check_version(git_version: &Version, check: &str, feature: &str) -> Result<()> {
    if !VersionReq::parse(check)
        .with_context(|| format!("failed to parse version {}", check))?
        .matches(git_version)
    {
        bail!(
            "git version {} does not match {} required for {}",
            git_version,
            check,
            feature
        )
    }
    Ok(())
}
