#![deny(clippy::nursery, clippy::cargo)]
use args::*;
mod args;

use errors::*;
mod errors;

use git::*;
mod git;

use structopt::StructOpt;

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{exit, Child, Command, Stdio};
use std::{env, io, str};

const DEFAULT_LIST_FORMAT: &str =
    "%C(yellow)%h%C(reset) %s %C(cyan)<%an>%C(reset) %C(green)(%cr)%C(reset)%C(auto)%d%C(reset)";

struct MenuCommand {
    command: String,
    args: Vec<String>,
}

impl MenuCommand {
    fn new(command: String, args: Vec<String>) -> Self {
        Self { command, args }
    }
}

fn run(args: Args) -> Result<()> {
    let format = match args.format {
        None => DEFAULT_LIST_FORMAT,
        Some(ref format) => format,
    };

    let toplevel = git_toplevel()?.context("failed to get git toplevel path")?;
    env::set_current_dir(&toplevel)?;

    let mut staged_files = git_staged_files()?;
    if staged_files.is_empty() {
        writeln!(
            io::stderr(),
            "Changes not staged for commit\nUse git add -p to stage changed files"
        )
        .ok();
        exit(1);
    }

    let mut cmd_sk = match args.list {
        false => Option::Some(spawn_menu().context("failed to spawn menu command")?),
        true => Option::None,
    };

    let range = git_rev_range(args.local)?.ok_or_else(|| {
        writeln!(io::stderr(), "No local commits found\nTry --all or set smash.onlylocal=false to list published commits").ok();
        exit(1);
    }).unwrap();

    let mut cmd_file_revs = spawn_file_revs(&range, &mut staged_files, args.commits)?;

    let stdout = cmd_file_revs
        .stdout
        .as_mut()
        .context("failed to acquire stdout from git log command")?;
    let stdout_reader = BufReader::new(stdout);
    let stdout_lines = stdout_reader.lines();

    for file_rev in stdout_lines {
        let line = file_rev?;
        let line = line
            .split_whitespace()
            .next()
            .context("failed to split commit hash from input line")?;
        let target = format_target(line, format)?;

        if args.list {
            let mut stdout = io::stdout();
            if stdout.write_all(&target).is_err() {
                return Ok(());
            }
        } else {
            if let Some(ref mut cmd_sk) = cmd_sk {
                if let Some(ref mut stdin) = cmd_sk.stdin {
                    if stdin.write(&target).is_err() {
                        break;
                    }
                }
            }
        }
    }
    cmd_file_revs.kill()?;

    if let Some(cmd_sk) = cmd_sk {
        let output = cmd_sk.wait_with_output()?;
        let target = select_target(output.stdout.as_ref())?;

        if target.is_empty() {
            return Ok(());
        }

        if !is_valid_git_rev(&target)? {
            bail!("Selected commit '{}' not found\nPossibly --format or smash.format doesn't return a hash", target);
        }

        if args.select {
            writeln!(io::stdout(), "{}", &target).ok();
            return Ok(());
        }

        git_commit_fixup(&target)?;

        if !args.no_rebase {
            git_rebase(&target, args.interactive)?;
        }
    }

    Ok(())
}

fn git_rebase(rev: &str, interactive: bool) -> Result<()> {
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

fn git_rev_root() -> Result<String> {
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

fn git_rev_range(local_only: bool) -> Result<Option<String>> {
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

fn git_rev_parse(rev: &str) -> Result<Option<String>> {
    git_rev_parse_stderr(rev, Stdio::piped())
}

fn git_rev_parse_stderr<T: Into<Stdio>>(rev: &str, stderr: T) -> Result<Option<String>> {
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

fn git_toplevel() -> Result<Option<PathBuf>> {
    Ok(git_rev_parse_stderr("--show-toplevel", Stdio::inherit())?.map(|e| PathBuf::from(e)))
}

fn is_valid_git_rev(rev: &str) -> Result<bool> {
    let files_args = vec!["rev-parse", "--verify", rev];
    let mut cmd = Command::new("git")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(&files_args)
        .spawn()?;
    Ok(cmd.wait()?.success())
}

fn git_commit_fixup(target: &str) -> Result<()> {
    let files_args = vec!["commit", "--no-edit", "--fixup", target];
    let output = Command::new("git").args(&files_args).output()?;
    if !output.status.success() {
        exit(output.status.code().unwrap_or_else(|| 1));
    }
    Ok(())
}

fn git_staged_files() -> Result<Vec<String>> {
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

fn spawn_file_revs(range: &str, staged_files: &mut Vec<String>, max_count: u32) -> Result<Child> {
    let mut file_revs_args = vec![
        "log",
        "--invert-grep",
        "--extended-regexp",
        "--grep",
        "^(fixup|squash)! .*$",
        "--format=%H %s",
        range,
    ]
    .into_iter()
    .map(|e| e.to_string())
    .collect::<Vec<_>>();
    if max_count > 0 {
        file_revs_args.push(format!("-{}", max_count).to_string());
    }
    file_revs_args.push("--".to_string());
    file_revs_args.append(staged_files);

    Ok(Command::new("git")
        .args(&file_revs_args)
        .stdout(Stdio::piped())
        .spawn()?)
}

fn spawn_menu() -> Result<Child> {
    let menu = resolve_menu_command()?;
    Ok(Command::new(menu.command)
        .args(menu.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?)
}

fn format_target(commit: &str, format: &str) -> Result<Vec<u8>> {
    let format = format!("--format={}", format);
    let args = vec!["--no-pager", "log", "-1", &format, commit];
    let output = Command::new("git")
        .stdout(Stdio::piped())
        .args(&args)
        .output()?;
    Ok(output.stdout)
}

fn select_target(line: &[u8]) -> Result<String> {
    let cow = String::from_utf8_lossy(line);
    Ok(cow
        .splitn(2, " ")
        .next()
        .context("failed to split first part of the target")?
        .into())
}

fn resolve_command(command: &str) -> Result<Option<String>> {
    let output = Command::new("sh")
        .stdout(Stdio::piped())
        .args(vec!["-c", format!("command -v {}", &command).as_ref()])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(
        String::from_utf8_lossy(&output.stdout).trim().to_owned(),
    ))
}

fn resolve_menu_command() -> Result<MenuCommand> {
    let fuzzy_args = vec![
        "--ansi".to_string(),
        "--preview".to_string(),
        "git show --stat --patch --color {+1}".to_string(),
    ];
    for cmd in &[("sk", &fuzzy_args), ("fzf", &fuzzy_args)] {
        if let Some(bin) = resolve_command(cmd.0)? {
            return Ok(MenuCommand::new(bin, cmd.1.to_owned()));
        }
    }
    bail!("Can't find any supported fuzzy matcher or menu command\nPlease install skim, fzf or configure one with smash.menu");
}

fn main() {
    let args = Args::from_args();

    if let Err(err) = run(args) {
        eprintln!("Error: {}", err);
        for cause in err.chain().skip(1) {
            eprintln!("{}", cause);
        }
        exit(1);
    }
}
