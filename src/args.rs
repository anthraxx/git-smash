use crate::errors::*;

use std::io::stdout;

use structopt::clap::{AppSettings, Shell};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about="Smash staged changes into previous commits.", global_settings = &[AppSettings::ColoredHelp, AppSettings::DeriveDisplayOrder])]
pub struct Args {
    /// List mode to print all potential targets to stdout
    #[structopt(long, group = "mode")]
    pub list: bool,
    /// Select mode to print final target to stdout
    #[structopt(long, group = "mode")]
    pub select: bool,
    /// Git log format to pretty print the targets
    #[structopt(long)]
    pub format: Option<String>,
    /// Limit number of listed commits (0 for unlimited stream)
    #[structopt(long, short = "n")]
    pub max_count: Option<u32>,
    /// List all revs including already published commits
    #[structopt(long, short = "a", group = "rev_range")]
    pub all: bool,
    /// Limit the listed revs to local commits
    #[structopt(long, short = "l", group = "rev_range")]
    pub local: bool,
    /// Rebase the fixup commit into the target
    #[structopt(long, group = "autorebase")]
    pub rebase: bool,
    /// Do not rebase the fixup commit into the target
    #[structopt(long, group = "autorebase")]
    pub no_rebase: bool,
    /// Let the user edit the list of commits before rebasing
    #[structopt(long)]
    pub interactive: bool,
    /// List commits acquired from blame chunks
    #[structopt(long)]
    pub blame: bool,
    /// Do not list commits acquired from blame chunks
    #[structopt(long)]
    pub no_blame: bool,
    /// List commits acquired from history of changed files
    #[structopt(long)]
    pub files: bool,
    /// Do not list commits acquired from history of changed files
    #[structopt(long)]
    pub no_files: bool,
    /// Limit the listed commits to the given range
    #[structopt(long, group = "rev_range")]
    pub range: Option<String>,
    #[structopt(subcommand)]
    pub subcommand: Option<SubCommand>,
}

#[derive(Debug, StructOpt)]
pub enum SubCommand {
    /// Generate shell completions
    #[structopt(name = "completions")]
    Completions(Completions),
}

#[derive(Debug, StructOpt)]
pub struct Completions {
    #[structopt(possible_values=&Shell::variants())]
    pub shell: Shell,
}

pub fn gen_completions(args: &Completions) -> Result<()> {
    Args::clap().gen_completions_to("git-smash", args.shell, &mut stdout());
    Ok(())
}
