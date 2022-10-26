use std::io::stdout;

use clap::CommandFactory;
use clap::{Args as ClapArgs, Parser, Subcommand};
use clap_complete::{generate, Shell};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    /// List mode to print all potential targets to stdout
    #[arg(long, group = "mode")]
    pub list: bool,
    /// Select mode to print final target to stdout
    #[arg(long, group = "mode")]
    pub select: bool,
    /// Git log format to pretty print the targets
    #[arg(long)]
    pub format: Option<String>,
    /// Limit number of listed commits (0 for unlimited stream)
    #[arg(long, short = 'n', value_name = "number")]
    pub max_count: Option<u32>,
    /// List all revs including already published commits
    #[arg(long, short = 'a', group = "rev_range")]
    pub all: bool,
    /// Limit the listed revs to local commits
    #[arg(long, short = 'l', group = "rev_range")]
    pub local: bool,
    /// Rebase the fixup commit into the target
    #[arg(long, group = "autorebase")]
    pub rebase: bool,
    /// Do not rebase the fixup commit into the target
    #[arg(long, group = "autorebase")]
    pub no_rebase: bool,
    /// Let the user edit the list of commits before rebasing
    #[arg(long, short = 'i')]
    pub interactive: bool,
    /// List commits acquired from blame chunks
    #[arg(long, group = "list_blame")]
    pub blame: bool,
    /// Do not list commits acquired from blame chunks
    #[arg(long, group = "list_blame")]
    pub no_blame: bool,
    /// List commits acquired from history of changed files
    #[arg(long, group = "list_files")]
    pub files: bool,
    /// Do not list commits acquired from history of changed files
    #[arg(long, group = "list_files")]
    pub no_files: bool,
    /// List commits acquired from recent history (default 0)
    #[arg(long, value_name = "number")]
    pub recent: Option<u32>,
    /// Limit the listed commits to the given range
    #[arg(long, group = "rev_range", value_name = "revision-range")]
    pub range: Option<String>,
    /// Smash staged changes and refine the log message
    #[arg(long, group = "fixup_mode")]
    pub amend: bool,
    /// Refine the log message ignoring all staged changes
    #[arg(long, group = "fixup_mode")]
    pub reword: bool,
    /// Target commit to smash into
    pub commit: Option<String>,
    #[command(subcommand)]
    pub subcommand: Option<SubCommand>,
}

#[derive(Debug, Subcommand)]
pub enum SubCommand {
    /// Generate shell completions
    #[clap(name = "completions")]
    Completions(Completions),
}

#[derive(Debug, ClapArgs)]
pub struct Completions {
    pub shell: Shell,
}

pub fn gen_completions(completions: &Completions) {
    let mut cmd = Args::command();
    let bin_name = cmd.get_name().to_string();
    generate(completions.shell, &mut cmd, &bin_name, &mut stdout());
}
