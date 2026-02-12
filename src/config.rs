#![allow(clippy::use_self)]
use crate::args::Args;
use crate::errors::*;
use crate::git::{git_check_version, git_version, GitConfigBuilder};

use std::str::FromStr;

use strum_macros::{Display, EnumString};

pub const DEFAULT_LIST_FORMAT: &str =
    "%C(yellow)%h%C(reset) [%(smash:source)] %s %C(cyan)<%an>%C(reset) %C(green)(%cr)%C(reset)%C(auto)%d%C(reset)";
pub const DEFAULT_FORMAT_SOURCE_FILES: &str = "%C(green)F%C(reset)";
pub const DEFAULT_FORMAT_SOURCE_BLAME: &str = "%C(red)B%C(reset)";
pub const DEFAULT_FORMAT_SOURCE_RECENT: &str = "%C(magenta)R%C(reset)️️";

#[derive(Debug, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum DisplayMode {
    Smash,
    List,
    Select,
}

pub enum CommitRange {
    Local,
    All,
    Range(String),
}

pub enum FixupMode {
    Fixup,
    Amend,
    Reword,
}

impl FixupMode {
    pub fn to_cli_option(&self, target: &str) -> String {
        match self {
            FixupMode::Fixup => format!("--fixup={}", target),
            FixupMode::Amend => format!("--fixup=amend:{}", target),
            FixupMode::Reword => format!("--fixup=reword:{}", target),
        }
    }
}

pub struct Config {
    pub mode: DisplayMode,
    pub range: CommitRange,
    pub format: String,
    pub max_count: u32,
    pub auto_rebase: bool,
    pub interactive: bool,
    pub blame: bool,
    pub files: bool,
    pub recent: u32,
    pub commit: Option<String>,
    pub source_label_files: String,
    pub source_label_blame: String,
    pub source_label_recent: String,
    pub fixup_mode: FixupMode,
    pub gpg_sign_option: Option<String>,
    pub verify_option: Option<String>,
    pub ext_diff: Option<String>,
}

impl Config {
    #[allow(clippy::cognitive_complexity)]
    pub fn load(args: &Args) -> Result<Self> {
        let git_version = git_version().context("failed to get git version")?;

        let config = Self {
            mode: if args.list {
                DisplayMode::List
            } else if args.select {
                DisplayMode::Select
            } else if let Some(mode) = GitConfigBuilder::new("smash.mode").get()? {
                DisplayMode::from_str(&mode)
                    .with_context(|| format!("failed to parse smash.mode '{}'", mode))?
            } else {
                DisplayMode::Smash
            },
            range: if args.local {
                CommitRange::Local
            } else if args.all {
                CommitRange::All
            } else if let Some(range) = &args.range {
                CommitRange::Range(range.into())
            } else if let Some(range) = GitConfigBuilder::new("smash.range").get()? {
                match range.as_str() {
                    "local" => CommitRange::Local,
                    "all" => CommitRange::All,
                    range => CommitRange::Range(range.into()),
                }
            } else {
                CommitRange::All
            },
            format: if let Some(format) = &args.format {
                format.into()
            } else {
                GitConfigBuilder::new("smash.format")
                    .get()?
                    .unwrap_or_else(|| DEFAULT_LIST_FORMAT.into())
            },
            max_count: if let Some(max_count) = args.max_count {
                max_count
            } else {
                GitConfigBuilder::new("smash.maxCommitCount")
                    .with_type("int")
                    .with_default("0")
                    .get_as_int()?
                    .unwrap_or(0)
            },
            auto_rebase: if args.rebase {
                true
            } else if args.no_rebase {
                false
            } else {
                GitConfigBuilder::new("smash.autorebase")
                    .with_type("bool")
                    .with_default("true")
                    .get_as_bool()?
                    .unwrap_or(true)
            },
            interactive: if args.interactive {
                true
            } else {
                GitConfigBuilder::new("smash.interactive")
                    .with_type("bool")
                    .with_default("false")
                    .get_as_bool()?
                    .unwrap_or(false)
            },
            blame: if args.blame {
                true
            } else if args.no_blame {
                false
            } else {
                GitConfigBuilder::new("smash.blame")
                    .with_type("bool")
                    .with_default("true")
                    .get_as_bool()?
                    .unwrap_or(true)
            },
            files: if args.files {
                true
            } else if args.no_files {
                false
            } else {
                GitConfigBuilder::new("smash.files")
                    .with_type("bool")
                    .with_default("true")
                    .get_as_bool()?
                    .unwrap_or(true)
            },
            recent: if let Some(recent) = args.recent {
                recent
            } else {
                GitConfigBuilder::new("smash.recent")
                    .with_type("int")
                    .with_default("0")
                    .get_as_int()?
                    .unwrap_or(0)
            },
            source_label_files: GitConfigBuilder::new("smash.filesSourceFormat")
                .with_default(DEFAULT_FORMAT_SOURCE_FILES)
                .get()?
                .unwrap_or_else(|| DEFAULT_FORMAT_SOURCE_FILES.into()),
            source_label_blame: GitConfigBuilder::new("smash.blameSourceFormat")
                .with_default(DEFAULT_FORMAT_SOURCE_BLAME)
                .get()?
                .unwrap_or_else(|| DEFAULT_FORMAT_SOURCE_BLAME.into()),
            source_label_recent: GitConfigBuilder::new("smash.recentSourceFormat")
                .with_default(DEFAULT_FORMAT_SOURCE_RECENT)
                .get()?
                .unwrap_or_else(|| DEFAULT_FORMAT_SOURCE_RECENT.into()),
            commit: args.commit.clone(),
            fixup_mode: if args.amend {
                git_check_version(&git_version, ">=2.33", "--amend")?;
                FixupMode::Amend
            } else if args.reword {
                git_check_version(&git_version, ">=2.33", "--reword")?;
                FixupMode::Reword
            } else {
                FixupMode::Fixup
            },
            gpg_sign_option: args.gpg_sign.as_ref().map_or_else(
                || {
                    if args.no_gpg_sign {
                        Some("--no-gpg-sign".into())
                    } else {
                        None
                    }
                },
                |gpg_sign| match gpg_sign.is_empty() {
                    true => Some("--gpg-sign".into()),
                    false => Some(format!("--gpg-sign={}", gpg_sign)),
                },
            ),
            verify_option: if args.verify {
                Some("--verify".into())
            } else if args.no_verify {
                Some("--no-verify".into())
            } else {
                None
            },
            ext_diff: if args.ext_diff {
                Some("--ext-diff".into())
            } else if args.no_ext_diff {
                Some("--no-ext-diff".into())
            } else {
                None
            },
        };

        Ok(config)
    }
}
