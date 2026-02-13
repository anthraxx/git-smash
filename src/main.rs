#![deny(clippy::nursery, clippy::cargo)]
use args::*;
mod args;

use errors::*;
mod errors;

use git::*;
mod git;

use config::*;
mod config;

use hash::*;
mod hash;

use std::collections::HashSet;
use std::hash::BuildHasherDefault;
use std::io::{BufRead, BufReader, Write};
use std::process::{exit, Child, Command, Stdio};
use std::{env, io, str};

use ahash::RandomState;
use clap::Parser;
use regex::Regex;

struct MenuCommand {
    command: String,
    args: Vec<String>,
}

impl MenuCommand {
    const fn new(command: String, args: Vec<String>) -> Self {
        Self { command, args }
    }
}

fn run(args: Args) -> Result<()> {
    if let Some(SubCommand::Completions(completions)) = args.subcommand {
        args::gen_completions(&completions);
        return Ok(());
    }

    let hasher = RandomState::new();
    let mut unique = HashSet::<u64, BuildHasherDefault<IdentityHasher>>::default();

    let config = Config::load(&args)?;

    let toplevel = git_toplevel().context("failed to get git toplevel path")?;
    env::set_current_dir(toplevel)?;

    let mut staged_files = git_staged_files()?;
    if staged_files.is_empty() {
        writeln!(
            io::stderr(),
            "Changes not staged for commit\nUse git add -p to stage changed files"
        )
        .ok();
        exit(1);
    }

    let range = git_rev_range(&config)?.ok_or_else(|| {
        writeln!(io::stderr(), "No local commits found\nTry --all or set smash.range=all to list published commits").ok();
        exit(1);
    }).unwrap();
    // Make sure the range is a valid rev expression
    if git_rev_parse(&range).is_err() {
        bail!("Ambiguous argument '{}': unknown revision", range)
    }

    if let Some(target) = config.commit {
        match git_rev_parse(&target) {
            Err(_) => bail!("Ambiguous argument '{}': unknown revision", target),
            Ok(target) => {
                git_commit_fixup(
                    &target,
                    config.fixup_mode,
                    &config.gpg_sign_option,
                    &config.verify_option,
                )?;

                if config.auto_rebase {
                    git_rebase(
                        &target,
                        config.interactive,
                        &config.gpg_sign_option,
                        &config.verify_option,
                    )?;
                }

                return Ok(());
            }
        }
    }

    let mut cmd_sk = match config.mode {
        DisplayMode::List => None,
        _ => Some(spawn_menu(&config).context("failed to spawn menu command")?),
    };

    if config.recent > 0 {
        for rev in git_rev_list(&range, config.recent)
            .with_context(|| format!("failed to get rev-list for {}", range))?
        {
            if !unique.insert(hash(&hasher, &rev)) {
                continue;
            }

            let target = format_target(&rev, &config.format, &config.source_label_recent)?;

            if !process_target(&target, &config.mode, &mut cmd_sk) {
                break;
            }
        }
    }

    if config.blame {
        let commits_from_blame = match config.blame {
            true => get_commits_from_blame(&staged_files, &range)?,
            false => Vec::new(),
        };
        for rev in commits_from_blame {
            if !unique.insert(hash(&hasher, &rev)) {
                continue;
            }

            let target = format_target(&rev, &config.format, &config.source_label_blame)?;

            if !process_target(&target, &config.mode, &mut cmd_sk) {
                break;
            }
        }
    }

    if config.files {
        let mut cmd_file_revs = spawn_file_revs(
            &mut staged_files,
            &config.format,
            &range,
            config.max_count,
            &config.source_label_files,
        )?;

        let stdout = cmd_file_revs
            .stdout
            .as_mut()
            .context("failed to acquire stdout from git log command")?;
        let stdout_reader = BufReader::new(stdout);
        let stdout_lines = stdout_reader.split(b'\n');

        for target in stdout_lines {
            let target = target.context("failed to read bytes from stream")?;
            let target = String::from_utf8_lossy(&target);
            let target = target.trim_end();
            let mut target = target.splitn(2, ' ');
            let target_hash = target.next().context("failed to extract target hash")?;
            let target = target.next().context("failed to extract target")?;

            if !unique.insert(hash(&hasher, &target_hash)) {
                continue;
            }

            if !process_target(target, &config.mode, &mut cmd_sk) {
                break;
            }
        }

        cmd_file_revs.kill()?;
    }

    if let Some(cmd_sk) = cmd_sk {
        let output = cmd_sk.wait_with_output()?;
        let target = select_target(output.stdout.as_ref())?;

        if target.is_empty() {
            return Ok(());
        }

        if !is_valid_git_rev(&target)? {
            bail!("Selected commit '{}' not found\nPossibly --format or smash.format doesn't return a hash", target);
        }

        if config.mode == DisplayMode::Select {
            writeln!(io::stdout(), "{}", &target).ok();
            return Ok(());
        }

        git_commit_fixup(
            &target,
            config.fixup_mode,
            &config.gpg_sign_option,
            &config.verify_option,
        )?;

        if config.auto_rebase {
            git_rebase(
                &target,
                config.interactive,
                &config.gpg_sign_option,
                &config.verify_option,
            )?;
        }
    }

    Ok(())
}

fn process_target(target: &str, mode: &DisplayMode, cmd_sk: &mut Option<Child>) -> bool {
    match mode {
        DisplayMode::List => {
            let mut stdout = io::stdout();
            if writeln!(stdout, "{}", target).is_err() {
                exit(0);
            }
        }
        _ => {
            if let Some(ref mut cmd_sk) = cmd_sk {
                if let Some(ref mut stdin) = cmd_sk.stdin {
                    if writeln!(stdin, "{}", &target).is_err() {
                        return false;
                    }
                }
            }
        }
    }
    true
}

fn get_commits_from_blame(staged_files: &[String], range: &str) -> Result<Vec<String>> {
    let mut diff_args = vec![
        "--no-pager",
        "diff",
        "--color=never",
        "--unified=1",
        "--no-prefix",
        "--cached",
        "--no-ext-diff",
    ]
    .into_iter()
    .map(|e| e.to_string())
    .collect::<Vec<_>>();
    diff_args.push("--".to_string());
    let mut staged_files = staged_files.to_owned();
    diff_args.append(&mut staged_files);

    let cmd_diff = Command::new("git")
        .args(&diff_args)
        .stdout(Stdio::piped())
        .spawn()?;

    let output = cmd_diff.wait_with_output()?;

    let re_split = Regex::new(r"(?m)^diff ")?;
    let re_file = Regex::new(r"(?m)^--- (.+)")?;
    let re_chunk = Regex::new(r"(?m)^@@ -([0-9]+)(,([0-9]+))? ([^ ]+) @@")?;

    let diff = String::from_utf8_lossy(&output.stdout);

    let mut commits: Vec<String> = Vec::new();

    for split in re_split.split(&diff).skip(1) {
        let file = re_file
            .captures(split)
            .context("failed to match file in chunk")?;
        let file = file.get(1).context("failed to get file group")?.as_str();
        if file == "/dev/null" {
            continue;
        }

        let mut blame_args = vec![
            "--no-pager".to_string(),
            "blame".to_string(),
            "--no-abbrev".to_string(),
            "-s".to_string(),
        ];

        for chunks in re_chunk.captures_iter(split) {
            let offset = chunks
                .get(1)
                .context("failed to get offset group")?
                .as_str();
            let length = chunks
                .get(3)
                .map(|m| m.as_str())
                .context("failed to get length group")
                .unwrap_or("1");

            let location = format!("{},+{}", offset, length);
            blame_args.push("-L".to_string());
            blame_args.push(location);
        }

        blame_args.push(range.to_string());
        blame_args.push("--".to_string());
        blame_args.push(file.to_string());

        let blame_output = Command::new("git")
            .args(blame_args)
            .stdout(Stdio::piped())
            .output()?;

        let blame_output = String::from_utf8_lossy(&blame_output.stdout);
        let split_commits: Vec<_> = blame_output
            .lines()
            .filter_map(|e| e.split_whitespace().next())
            .collect();
        for hash in split_commits {
            if hash.starts_with('^') {
                continue;
            }
            commits.push(hash.into());
        }
    }

    Ok(commits)
}

fn spawn_file_revs(
    staged_files: &mut Vec<String>,
    format: &str,
    range: &str,
    max_count: u32,
    source_format: &str,
) -> Result<Child> {
    let format = format.replace("%(smash:source)", source_format);
    let mut file_revs_args = vec![
        "--no-pager",
        "log",
        "--color",
        "--invert-grep",
        "--extended-regexp",
        "--grep",
        "^(fixup|squash)! .*$",
        format!("--format=%H {}", format).as_str(),
        range,
    ]
    .into_iter()
    .map(|e| e.to_string())
    .collect::<Vec<_>>();
    if max_count > 0 {
        file_revs_args.push(format!("-{}", max_count));
    }
    file_revs_args.push("--".to_string());
    file_revs_args.append(staged_files);

    Ok(Command::new("git")
        .args(&file_revs_args)
        .stdout(Stdio::piped())
        .spawn()?)
}

fn format_target(commit: &str, format: &str, source_format: &str) -> Result<String> {
    let format = format.replace("%(smash:source)", source_format);
    let format = format!("--format={}", format);
    let args = vec!["--no-pager", "log", "--color", "-1", &format, commit];
    let output = Command::new("git")
        .stdout(Stdio::piped())
        .args(&args)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn spawn_menu(config: &Config) -> Result<Child> {
    let menu = resolve_menu_command(config)?;
    Ok(Command::new(menu.command)
        .args(menu.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .env("DFT_COLOR", "always")
        .spawn()?)
}

fn select_target(line: &[u8]) -> Result<String> {
    let cow = String::from_utf8_lossy(line);
    Ok(cow
        .split(' ')
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

fn resolve_menu_command(config: &Config) -> Result<MenuCommand> {
    let pipe = config.pager.as_ref().map_or_else(
        || "".to_string(),
        |pager| {
            if pager == "delta" {
                "| delta".to_string()
            } else {
                "".to_string()
            }
        },
    );
    let ext_diff = config.ext_diff.clone().unwrap_or_default();
    let show_args = [ext_diff].join(" ");

    let fuzzy_args = vec![
        "--ansi".to_string(),
        "--bind".to_string(),
        "ctrl-f:preview-page-down,ctrl-b:preview-page-up".to_string(),
        "--preview".to_string(),
        format!("git show --stat --patch --color {show_args} {{1}}{pipe}"),
    ];
    for cmd in &[("fzf", &fuzzy_args)] {
        if let Some(bin) = resolve_command(cmd.0)? {
            return Ok(MenuCommand::new(bin, cmd.1.to_owned()));
        }
    }
    bail!("Can't find any supported fuzzy matcher or menu command\nPlease install fzf or configure one with smash.menu");
}

fn main() {
    let args = Args::parse();

    if let Err(err) = run(args) {
        eprintln!("Error: {}", err);
        for cause in err.chain().skip(1) {
            eprintln!("{}", cause);
        }
        exit(1);
    }
}
