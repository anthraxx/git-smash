use args::*;
mod args;

use structopt::StructOpt;

use std::str;
use std::process::{Command, Stdio, exit, Child};
use std::io::{BufReader, BufRead, Write};

use anyhow::{Result};
use regex::Regex;

fn run(args: Args) -> Result<()> {
    let re_fixup = Regex::new(r"^[a-f0-9]+ (fixup|squash)! ")?;

    let format = match args.format {
        None => "%C(yellow)%h%C(reset) %s %C(cyan)<%an>%C(reset) %C(green)(%cr)%C(reset)%C(auto)%d%C(reset)",
        Some(ref format) => format,
    };

    let staged_files = git_staged_files()?;
    if staged_files.is_empty() {
        eprintln!("Changes not staged for commit\nUse git add -p to stage changed files");
        return Ok(());
    }

    let mut cmd_sk = match args.list {
        false => Option::Some(spawn_menu()?),
        true => Option::None,
    };

    let git_bin = "git";

    'files: for filename in staged_files {
        let file_revs_args = vec!["log", "--format=%H %s", "HEAD", "--", &filename];
        let mut cmd_file_revs = Command::new(&git_bin)
            .args(&file_revs_args)
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = cmd_file_revs.stdout.as_mut().unwrap();
        let stdout_reader = BufReader::new(stdout);
        let stdout_lines = stdout_reader.lines();

        let mut count = 0;
        for file_rev in stdout_lines {
            let line = file_rev?;
            if re_fixup.is_match(&line) {
                continue;
            }
            let line = line.splitn(2, " ").next().unwrap();

            let target = format_target(&line, &format)?;

            if args.list {
                print!("{}", String::from_utf8_lossy(&target));
            } else {
                if let Some(ref mut cmd_sk) = cmd_sk {
                    if let Some(ref mut stdin) = cmd_sk.stdin {
                        if stdin.write(&target).is_err() {
                            break 'files;
                        }
                    }
                }
            }

            count += 1;
            if count >= args.commits {
                break;
            }
        }
        cmd_file_revs.kill()?;
    }

    if let Some(cmd_sk) = cmd_sk {
        let output = cmd_sk.wait_with_output()?;
        let target = select_target(output.stdout.as_ref());
        if !target.is_empty() {
            if args.select {
                println!("{}", target);
                return Ok(());
            }

            git_commit_fixup(&target)?;
        }
    }

    Ok(())
}

fn git_commit_fixup(target: &str) -> Result<()> {
    let git_bin = "git";
    let files_args = vec!["commit", "--no-edit", "--fixup", &target];
    let mut cmd_commit = Command::new(&git_bin)
        .stdout(Stdio::piped())
        .args(&files_args)
        .spawn()?;

    let stdout = cmd_commit.stdout.as_mut().unwrap();
    let stdout_reader = BufReader::new(stdout);
    let stdout_lines = stdout_reader.lines();
    for line in stdout_lines {
        println!("{}", line?);
    }
    Ok(())
}

fn git_staged_files() -> Result<Vec<String>> {
    let git_bin = "git";
    let files_args = vec!["diff", "--color=never", "--name-only", "--cached"];
    let mut cmd_files = Command::new(&git_bin)
        .stdout(Stdio::piped())
        .args(&files_args)
        .spawn()?;
    let stdout = cmd_files.stdout.as_mut().unwrap();
    let stdout_reader = BufReader::new(stdout);
    Ok(stdout_reader.lines().filter_map(|e| e.ok()).collect())
}

fn spawn_menu() -> Result<Child> {
    let sk_bin = "sk";
    let sk_args = vec!["--ansi", "--preview", "git show --stat --patch --color {+1}"];
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
    let mut cmd = Command::new(&git_bin)
        .stdout(Stdio::piped())
        .args(&args)
        .spawn()?;
    cmd.wait()?;
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
