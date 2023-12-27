
use anyhow::Result;
use clap::{Command, Arg, ArgMatches};
use crate::svn;
use crate::util::{self, StringWrapper};
use crate::util::SvError::*;
use std::path::Path;
use std::fmt::Display;
use super::SvCommand;

pub struct Ignore;
struct Options {
    path:        String,
}

impl Options {
    fn build_options(matches: &ArgMatches) -> Options {
        let path = matches.get_one::<String>("path").map(|s| s.clone()).unwrap_or(".".to_string());
        Options { path }
    }
}

impl SvCommand for Ignore {
    fn name(&self) -> &'static str { "ignore" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Write svn:ignore properties to stdout in .gitignore format")
            .after_help("PATH must refer to a working directory (not a repository URL.\n\
                        If PATH is ommitted the current working directory is used by default."
            )
            .arg(
                Arg::new("path")
                .value_name("PATH")
                .help("Limit commits to given paths (default is '.')")
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        Ignore::write_ignore_entries(&Options::build_options(matches))
    }
}

impl Ignore {
    
    fn is_directory<S>(path: S) -> bool
        where S: AsRef<str> + Display {
         Path::new(path.as_ref()).is_dir()
    }

    fn is_working_directory(path: &String) -> Result<bool> {
        let info = svn::info(path, None)?;
        Ok(info.wc_path.is_some() && info.kind == "dir")
   }

    fn get_ignores(path: &String, global: bool) -> Result<Option<String>> {
        let prop = (if global { "svn:global-ignores" } else { "svn:ignore" }).to_owned();
        let args = vec![
            "pget".to_owned(),
            prop,
            path.to_string()
        ];
        let output = svn::run_svn(&args, svn::CWD)?;
        if output.status.success() {
            Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()))
        }
        else {
            Ok(None)
        }
    
    }

    fn write_ignore_entries(options: &Options) -> Result<()> {
        let prefix_len = options.path.chomp('/').len() + 1; // Add one for trailing slash

        fn svn_ignore(dir_path: &String, prefix_len: usize) -> Result<()> {
            if let Some(ignore_output) = Ignore::get_ignores(dir_path, false)? {
                let ignores = ignore_output
                .split("\n")
                .map(|l| l.trim())  // Clean up and skip blank lines
                .filter(|l| !l.is_empty());

                for ignore in ignores {
                    let ignore_path  = util::join_paths(dir_path, ignore.to_owned().chomp('/'));
                    // We prefix each path with a slash so that it refers to the
                    // specific entry as per .gitignore rules.
                    // See: https://git-scm.com/docs/gitignore
                    if Ignore::is_directory(&ignore_path) {
                        println!("/{}/", &ignore_path[prefix_len..]);
                    } else {
                        println!("/{}", &ignore_path[prefix_len..]);
                    }                        
                }                
            }


            if let Some(ignore_output) = Ignore::get_ignores(dir_path, true)? {
                let global_ignores = ignore_output
                         .split("\n")
                         .map(|l| l.trim())  // Clean up and skip blank lines
                         .filter(|l| !l.is_empty());
                for global_ignore in global_ignores {
                    let base_path   = util::join_paths(dir_path, "**");
                    let ignore_path = util::join_paths(base_path, global_ignore.to_owned().chomp('/'));
                    // We prefix each path with a slash so that it refers to the
                    // specific entry as per .gitignore rules.
                    // See: https://git-scm.com/docs/gitignore
                    if Ignore::is_directory(&ignore_path) {
                        println!("/{}/", &ignore_path[prefix_len..]);
                    } else {
                        println!("/{}", &ignore_path[prefix_len..]);
                    }
                }
            }

            //  Recursively process all subdirectories
            let path_list = svn::path_list(dir_path.as_str())?;
            for sub_dir in &path_list.entries {
                if sub_dir.kind == "dir" {
                    let subdir_path = util::join_paths(dir_path, sub_dir.name.chomp('/'));
                    svn_ignore(&subdir_path, prefix_len)?;
                }
            }
            Ok(())
        }

        if !Ignore::is_working_directory(&options.path)? {
            let msg  = format!("{} is not a subversion working copy directory", options.path);
            Err(General(msg).into())
        }
        else {
            svn_ignore(&options.path, prefix_len)
        }
    }
}