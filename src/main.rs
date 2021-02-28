use args::*;
mod args;

use structopt::StructOpt;

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{exit, Child, Command, Stdio};
use std::{env, io, str};

use anyhow::{bail, Context, Result};

fn run(args: Args) -> Result<()> {
    let format = match args.format {
        None => "%C(yellow)%h%C(reset) %s %C(cyan)<%an>%C(reset) %C(green)(%cr)%C(reset)%C(auto)%d%C(reset)",
        Some(ref format) => format,
    };

    let toplevel = git_toplevel()?.context("failed to get git toplevel path")?;
    env::set_current_dir(&toplevel)?;

    let mut staged_files = git_staged_files()?;
    if staged_files.is_empty() {
        if writeln!(
            io::stderr(),
            "Changes not staged for commit\nUse git add -p to stage changed files"
        )
        .is_err()
        {}
        exit(1);
    }

    let mut cmd_sk = match args.list {
        false => Option::Some(spawn_menu()?),
        true => Option::None,
    };

    let git_bin = "git";
    let range = git_rev_range(args.local)?.ok_or_else(|| {
        if writeln!(io::stderr(), "No local commits found\nTry --all or set smash.onlylocal=false to list published commits").is_err() {}
        exit(1);
    }).unwrap();

    let mut file_revs_args = vec![
        "log",
        "--invert-grep",
        "--extended-regexp",
        "--grep",
        "^(fixup|squash)! .*$",
        "--format=%H %s",
        &range,
    ]
    .into_iter()
    .map(|e| e.to_string())
    .collect::<Vec<_>>();
    if args.commits > 0 {
        file_revs_args.push(format!("-{}", args.commits).to_string());
    }
    file_revs_args.push("--".to_string());
    file_revs_args.append(&mut staged_files);

    let mut cmd_file_revs = Command::new(&git_bin)
        .args(&file_revs_args)
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = cmd_file_revs.stdout.as_mut().context("failed to acquire stdout from git log command")?;
    let stdout_reader = BufReader::new(stdout);
    let stdout_lines = stdout_reader.lines();

    for file_rev in stdout_lines {
        let line = file_rev?;
        let line = line.split_whitespace().next().context("failed to split commit hash from input line")?;
        let target = format_target(&line, &format)?;

        if args.list {
            if write!(io::stdout(), "{}", String::from_utf8_lossy(&target)).is_err() {
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
        let target = select_target(output.stdout.as_ref());

        if target.is_empty() {
            return Ok(());
        }

        if !is_valid_git_rev(&target)? {
            if writeln!(io::stderr(), "Selected commit '{}' not found\nPossibly --format or smash.format doesn't return a hash", target).is_err() {}
            exit(1);
        }

        if args.select {
            if writeln!(io::stdout(), "{}", &target).is_err() {}
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

    let git_bin = "git";
    let args = vec![
        "rebase",
        "--interactive",
        "--autosquash",
        "--autostash",
        &rev,
    ];
    let mut cmd = Command::new(&git_bin);
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
    let git_bin = "git";
    let args = vec!["rev-list", "--max-parents=0", "HEAD"];
    let cmd = Command::new(&git_bin)
        .stdout(Stdio::piped())
        .args(&args)
        .spawn()?;
    let output = cmd.wait_with_output()?;
    if !output.status.success() {
        bail!("failed to get git rev root");
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .into_owned()
        .trim_end()
        .to_owned())
}

fn git_rev_range(local_only: bool) -> Result<Option<String>> {
    let mut range = "HEAD";
    let upstream = git_rev_parse("@{upstream}")?;

    if local_only && upstream.is_some() {
        let upstream = upstream.unwrap();
        let head = git_rev_parse("HEAD")?.context("failed to rev parse HEAD")?;
        if upstream == head {
            return Ok(None);
        }
        range = "@{upstream}..HEAD";
    }

    Ok(Some(range.to_string()))
}

fn git_rev_parse(rev: &str) -> Result<Option<String>> {
    git_rev_parse_stderr(rev, Stdio::piped())
}

fn git_rev_parse_stderr<T: Into<Stdio>>(rev: &str, stderr: T) -> Result<Option<String>> {
    let git_bin = "git";
    let args = vec!["rev-parse", rev];
    let cmd = Command::new(&git_bin)
        .stdout(Stdio::piped())
        .stderr(stderr)
        .args(&args)
        .spawn()?;
    let output = cmd.wait_with_output()?;
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
    let git_bin = "git";
    let files_args = vec!["rev-parse", "--verify", &rev];
    let mut cmd = Command::new(&git_bin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(&files_args)
        .spawn()?;
    Ok(cmd.wait()?.success())
}

fn git_commit_fixup(target: &str) -> Result<()> {
    let git_bin = "git";
    let files_args = vec!["commit", "--no-edit", "--fixup", &target];
    let cmd_commit = Command::new(&git_bin).args(&files_args).spawn()?;
    let output = cmd_commit.wait_with_output()?;
    if !output.status.success() {
        exit(output.status.code().unwrap_or_else(|| 1));
    }
    Ok(())
}

fn git_staged_files() -> Result<Vec<String>> {
    let git_bin = "git";
    let files_args = vec!["diff", "--color=never", "--name-only", "--cached"];
    let cmd_files = Command::new(&git_bin)
        .stdout(Stdio::piped())
        .args(&files_args)
        .spawn()?;
    let output = cmd_files.wait_with_output()?;
    if !output.status.success() {
        exit(output.status.code().unwrap_or_else(|| 1));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .lines()
        .map(|e| e.to_owned())
        .collect())
}

fn spawn_menu() -> Result<Child> {
    let sk_bin = "sk";
    let sk_args = vec![
        "--ansi",
        "--preview",
        "git show --stat --patch --color {+1}",
    ];
    Ok(Command::new(&sk_bin)
        .args(&sk_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?)
}

fn format_target(commit: &str, format: &str) -> Result<Vec<u8>> {
    let git_bin = "git";
    let format = format!("--format={}", format);
    let args = vec!["--no-pager", "log", "-1", &format, &commit];
    let cmd = Command::new(&git_bin)
        .stdout(Stdio::piped())
        .args(&args)
        .spawn()?;
    let output = cmd.wait_with_output()?;
    Ok(output.stdout)
}

fn select_target(line: &[u8]) -> String {
    let cow = String::from_utf8_lossy(line);
    cow.splitn(2, " ").next().unwrap().into()
}

fn main() {
    let args = Args::from_args();

    if let Err(err) = run(args) {
        eprintln!("Error: {}", err);
        for cause in err.chain().skip(1) {
            eprintln!("Because: {}", cause);
        }
        exit(1);
    }
}
