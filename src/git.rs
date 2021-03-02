use std::process::{Command, Stdio, exit};

use crate::errors::*;
use std::path::PathBuf;

pub struct GitConfigBuilder {
    key: &'static str,
    default: Option<&'static str>,
    value_type: Option<&'static str>,
}

impl GitConfigBuilder {
    pub fn new(key: &'static str) -> Self {
        Self {
            key,
            default: None,
            value_type: None,
        }
    }

    pub fn with_default(mut self, default: &'static str) -> Self {
        self.default = Some(default);
        self
    }

    pub fn with_type(mut self, value_type: &'static str) -> Self {
        self.value_type = Some(value_type);
        self
    }

    pub fn get(&self) -> Result<String> {
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
            bail!("{}", String::from_utf8_lossy(output.stderr.as_ref()).trim());
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned())
    }

    pub fn get_as_bool(&mut self) -> Result<bool> {
        let value: bool = self
            .get()?
            .parse()
            .with_context(|| anyhow!("Failed to parse key '{}' as bool", self.key))?;
        Ok(value)
    }
}

pub fn git_rebase(rev: &str, interactive: bool) -> Result<()> {
    let root = git_rev_root()?;
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
    }
    let mut cmd = cmd.args(&args).spawn()?;
    let status = cmd.wait()?;

    if !status.success() {
        exit(status.code().unwrap_or_else(|| 1));
    }

    Ok(())
}

pub fn git_rev_root() -> Result<String> {
    let args = vec!["rev-list", "--max-parents=0", "HEAD"];
    let output = Command::new("git")
        .stdout(Stdio::piped())
        .args(&args)
        .output()?;
    if !output.status.success() {
        bail!("failed to get git rev root");
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .into_owned()
        .trim_end()
        .to_owned())
}

pub fn git_rev_range(local_only: bool) -> Result<Option<String>> {
    let head = "HEAD".to_string();

    if !local_only {
        return Ok(Some(head));
    }

    let upstream = git_rev_parse("@{upstream}")?;
    if let Some(upstream) = upstream {
        let head = git_rev_parse("HEAD")?.context("failed to rev parse HEAD")?;
        if upstream == head {
            return Ok(None);
        }
        return Ok(Some("@{upstream}..HEAD".to_string()));
    }

    Ok(Some(head))
}

pub fn git_rev_parse(rev: &str) -> Result<Option<String>> {
    git_rev_parse_stderr(rev, Stdio::piped())
}

pub fn git_rev_parse_stderr<T: Into<Stdio>>(rev: &str, stderr: T) -> Result<Option<String>> {
    let args = vec!["rev-parse", rev];
    let output = Command::new("git")
        .stdout(Stdio::piped())
        .stderr(stderr)
        .args(&args)
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(
        String::from_utf8_lossy(&output.stdout)
            .into_owned()
            .trim_end()
            .to_owned(),
    ))
}

pub fn git_toplevel() -> Result<Option<PathBuf>> {
    Ok(git_rev_parse_stderr("--show-toplevel", Stdio::inherit())?.map(|e| PathBuf::from(e)))
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

pub fn git_commit_fixup(target: &str) -> Result<()> {
    let files_args = vec!["commit", "--no-edit", "--fixup", target];
    let output = Command::new("git").args(&files_args).output()?;
    if !output.status.success() {
        exit(output.status.code().unwrap_or_else(|| 1));
    }
    Ok(())
}

pub fn git_staged_files() -> Result<Vec<String>> {
    let files_args = vec!["diff", "--color=never", "--name-only", "--cached"];
    let output = Command::new("git")
        .stdout(Stdio::piped())
        .args(&files_args)
        .output()?;
    if !output.status.success() {
        exit(output.status.code().unwrap_or_else(|| 1));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .lines()
        .map(|e| e.to_owned())
        .collect())
}
