
use clap::Parser;
use super::*;
use anyhow::Result;
use std::process;

/// Replay a bisect session from a log file.
///
/// You can save the current bisect session log using the `svu bisect log` command and redirecting
/// the output to a file.  Then, later, you can use `svu bisect replay` to reply the entire
/// bisect session from the saved log file.
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
)]
pub struct Replay {
    /// Path to log file.
    #[arg(num_args = 1..=1, required = true)]
    log_fiie: String,
}

impl Replay {
    pub fn run(&mut self) -> Result<()> {
        let wc_info = svn::workingcopy_info()?;  // Make sure we are in a working copy.
        let wc_root = PathBuf::from(wc_info.wc_path.unwrap());
    
        let cmd = process::Command::new("/bin/sh")
            .current_dir(wc_root)
            .args([self.log_fiie.clone()])
            .stdout(process::Stdio::inherit())
            .stderr(process::Stdio::inherit())
            .output()?;
    
        if cmd.status.success() {
            Ok(())
        } else {
            Err(General("Log replay did not finish successfully".to_string()).into())
        }
    }
}
