
use anyhow::Result;
use clap::Parser;
use crate::{svn, util};
use crate::util::SvError::*;


/// Display and configure repository prefixes.
/// 
/// The `branch` and `filerevs` subcommands depend on prefixes to know where the
/// branches and tags are located in your repository.  By default it is assumed that they
/// are located in the standard locations (^/trunk, ^/branches, ^/tags).  If you are using a
/// non-standard configuration or if you have multiple locations containing branches and/or tags
/// then you can use `branch prefix` to let svu know where to find them.
/// 
/// For example you may keep older branches in ^/obsolete-branches in order to keep your
/// ^/branches location less cluttered.
/// 
/// All prefixes must start with '^/'
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
    after_help = "\
    By default svu assumes that your repository is using the defacto prefixes (^/trunk, ^/branches/ ^/tags). \n\
    You can use this command to configure other prefixes so that the `branch` and `filerevs` commands can find them.\n\
    type `svu prefix --help` for more information."
)]
pub struct Prefix {
    /// Add a branch prefix.
    #[arg(long, value_name = "PREFIX", value_parser = parse_prefix)]
    add_branch: Vec<String>,

    /// Remove a branch prefix.
    #[arg(long, value_name = "PREFIX", value_parser = parse_prefix)]
    rem_branch: Vec<String>,

    /// Add a tag prefix.
    #[arg(long, value_name = "PREFIX", value_parser = parse_prefix)]
    add_tag: Vec<String>,

    /// Remove a tag prefix.
    #[arg(long, value_name = "PREFIX", value_parser = parse_prefix)]
    rem_tag: Vec<String>,

    /// Set the trunk prefix.
    #[arg(long, value_name = "PREFIX", value_parser = parse_prefix)]
    set_trunk: Option<String>,
}

impl Prefix {
    pub fn run(&mut self) -> Result<()> {
        let mut prefixes = svn::load_prefixes()?;
        let mut modified = false;

        if let Some(trunk_prefix) = &self.set_trunk {
            prefixes.trunk_prefix = trunk_prefix.clone();
            modified = true;
        }

        if !self.add_branch.is_empty() || !self.rem_branch.is_empty() {
            let to_add: Vec<String> = self
                .add_branch
                .iter()
                .filter(|a| !prefixes.branch_prefixes.contains(a))
                .cloned()
                .collect();
            prefixes.branch_prefixes.extend(to_add);

            prefixes
                .branch_prefixes
                .retain(|e| !self.rem_branch.contains(e));

            if prefixes.branch_prefixes.is_empty() {
                prefixes.branch_prefixes.push("branches".to_string());
            }
            modified = true;
        }

        if !self.add_tag.is_empty() || !self.rem_tag.is_empty() {
            let to_add: Vec<String> = self
                .add_tag
                .iter()
                .filter(|a| !prefixes.tag_prefixes.contains(a))
                .cloned()
                .collect();
            prefixes.tag_prefixes.extend(to_add);

            prefixes.tag_prefixes.retain(|e| !self.rem_tag.contains(e));

            if prefixes.tag_prefixes.is_empty() {
                prefixes.tag_prefixes.push("tags".to_string());
            }
            modified = true;
        }

        if modified {
            svn::save_prefixes(&prefixes)?;
        }

        let divider = util::divider(40);
        //  Finally display all of the configured prefixes to stdout.
        println!("Trunk prefix");
        println!("{}", divider);
        println!("^/{}", prefixes.trunk_prefix);

        println!("\nBranch prefixes");
        println!("{}", divider);
        let mut sorted = prefixes.branch_prefixes;
        sorted.sort();
        for prefix in &sorted {
            println!("^/{}", prefix);
        }

        println!("\nTag prefixes");
        println!("{}", divider);
        let mut sorted = prefixes.tag_prefixes;
        sorted.sort();
        for prefix in &sorted {
            println!("^/{}", prefix);
        }
        Ok(())
    }
}


fn parse_prefix(arg: &str) -> Result<String> {
    if !arg.starts_with("^/") {
        Err(General("Prefix must begin with '^/'".to_string()).into())
    } else if arg.len() == 2 {
        Err(General("Prefix cannot refer to the repository root".to_string()).into())
    } else {
        Ok(arg[2..].trim_end_matches('/').to_string())
    }
}
