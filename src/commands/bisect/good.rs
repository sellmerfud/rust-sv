
use clap::{Command, Arg, ArgMatches};
use super::*;
use anyhow::Result;

pub struct Good;
struct Options {
    revision: Option<String>,
}

impl BisectCommand for Good {
    fn name(&self) -> &'static str { "good" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Mark a revision as good  (It does not contain the bug)")
            .arg(
                Arg::new("revision")
                .value_name("REVISION")
                .help("The good revision.\n\
                       If not specified, the current working copy revision is used.")
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        do_work(&build_options(matches))?;
        Ok(())
    }
}

fn build_options(matches: &ArgMatches) -> Options {
    Options {
        revision: matches.get_one::<String>("revision").map(|s| s.to_string())
    }
}

fn do_work(_options: &Options) -> Result<()> {
    svn::working_copy_info()?;  // Make sure we are in a working copy.
    if true {
        Ok(())
    }
    else {
        Err(General("Failed..".to_string()).into())
    }
}