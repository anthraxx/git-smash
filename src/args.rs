use structopt::clap::{AppSettings};
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
}