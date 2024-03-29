
use thiserror::Error;
use crate::svn::{self, LogPath, FromPath, LogEntry};
use colored::*;
use chrono::{DateTime, Local, NaiveDateTime, Utc};
use std::sync::OnceLock;
use std::path::PathBuf;
use std::fs::{create_dir, rename};
use anyhow::Result;

#[derive(Error, Debug)]
pub enum SvError {
    #[error("{0}")]
    General(String),
    #[error("{}", String::from_utf8_lossy(&.0.stderr))]
    SvnError(std::process::Output),
}

pub fn join_paths<S, T>(base: S, leaf: T) -> String
where
S: AsRef<str>,
T: AsRef<str>,
{
    let mut path = String::new();

    path += base.as_ref().trim_end_matches('/');
    path += "/";
    path += leaf.as_ref().trim_matches('/');
    path
}

//  We create a .svu directory in the top directory of the working copy
//  This gives sv commands a place to store data
//  This will throw an error of the directory cannot be resloved.
//  Previously we use .sv as the directory name, so if we find a .sv
//  directory we rename it to .svu
pub fn data_directory() -> Result<PathBuf> {
    let wc_info = svn::workingcopy_info()?; // Make sure we are in a working copy.
    let wc_root = PathBuf::from(wc_info.wc_path.unwrap());
    let path = wc_root.join(".svu");
    if !path.is_dir() {
        let prev_path = wc_root.join(".sv");
        if prev_path.is_dir() {
            rename(&prev_path, &path)?;
        } else {
            create_dir(&path)?
        }
    }
    Ok(path)
}

pub fn formatted_log_path(log_path: &LogPath) -> String {
    let color = match log_path.action.as_str() {
        "D"  => "red",
        "A"  => "green",
        "M"  => "blue",
        _    => "white"
    };

    let base = format!(
        "  {} {}",
        log_path.action.color(color),
        log_path.path.color(color),
    );

    match &log_path.from_path {
        Some(FromPath { path, revision }) => {
            format!("{} (from {} {})", base, path.magenta(), revision.yellow())
        }
        None => base
    }
}

//  Create a `null` date value to use when an
//  entry has no date.
pub fn null_date() -> &'static DateTime<Local> {
    static NULL_DATE: OnceLock<DateTime<Local>> = OnceLock::new();
    NULL_DATE.get_or_init(|| {
        let timestamp_millis: i64 = -2208936075000; //Mon Jan 01 1900 14:38:45 GMT+0000
        let naive_datetime = NaiveDateTime::from_timestamp_millis(timestamp_millis).unwrap();
        let offset = *Local::now().offset();
        DateTime::<Local>::from_naive_utc_and_offset(naive_datetime, offset)
    })
}

pub fn parse_svn_date(date_str: &str) -> DateTime<Local> {
    DateTime::parse_from_rfc3339(date_str)
        .unwrap()  // We assume all svn dates are well formed!
        .with_timezone(&Local)
}

pub fn svn_date_to_rfc3339_string(date: &DateTime<Local>) -> String {
    let utc_date = date.with_timezone(&Utc);
    utc_date.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}

pub fn parse_svn_date_opt(opt_date_str: Option<String>) -> DateTime<Local> {
    if let Some(date_str) = opt_date_str {
        parse_svn_date(date_str.as_str())
    } else {
        *null_date()
    }
}

pub fn display_svn_date(date: &DateTime<Local>) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub fn display_svn_time(date: &DateTime<Local>) -> String {
    date.format("%H:%M:%S").to_string()
}

pub fn display_svn_datetime(date: &DateTime<Local>) -> String {
    if date == null_date() {
        "n/a".to_owned()
    } else {
        format!("{} {}", display_svn_date(date), display_svn_time(date))
    }
}

pub(crate) mod datetime_serializer {
    use chrono::{DateTime, Local};
    use serde::{self, Deserialize, Serializer, Deserializer};
    // use anyhow::Result;

    use super::{svn_date_to_rfc3339_string, parse_svn_date};

    pub fn serialize<S>(
        date: &DateTime<Local>,
        serializer: S,
    ) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = svn_date_to_rfc3339_string(date);
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> core::result::Result<DateTime<Local>, D::Error>
    where
        D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(parse_svn_date(&s))
    }

}

pub fn divider(len: usize) -> String {
    "-".repeat(len)
}

//  Print formatted commit info to stdout.
pub fn show_commit(log_entry: &LogEntry, show_msg: bool, show_paths: bool) {
    let divider = divider(70);
    println!("{}", divider);
    println!("Commit: {}", log_entry.revision.yellow());
    println!("Author: {}", log_entry.author.cyan());
    println!("Date  : {}", display_svn_datetime(&log_entry.date).magenta());
    println!("{}", divider);

    if show_msg {
        for line in &log_entry.msg {
            println!(" {}", line);
        }
    }
    println!();

    if !log_entry.paths.is_empty() {
        struct Totals{
            chg: u16,
            add: u16,
            del: u16,
            rep: u16,
        }
        let mut totals = Totals {
            chg: 0,
            add: 0,
            del: 0,
            rep: 0,
        };

        for path_entry in &log_entry.paths {
            match path_entry.action.as_str() {
                "M" => totals.chg += 1,
                "A" => totals.add += 1,
                "D" => totals.del += 1,
                "R" => totals.rep += 1,
                _   => ()
            }
        }
        let label = if totals.chg == 1 { "file" } else { "files" };
        let tot_line = format!(
            "{} {} modified, {} added, {} deleted, {} replaced",
            totals.chg, label, totals.add, totals.del, totals.rep
        );
        println!("{}", tot_line.cyan());
    }

    if show_paths {
        for path in &log_entry.paths {
            println!("{}", formatted_log_path(path))
        }
    }
}

pub fn print_diff_line(line: &str) {
    let color = if line.starts_with("---") || line.starts_with("+++") { "blue" }
           else if line.starts_with("Index:") || line.starts_with("==========") { "yellow" }
           else if line.starts_with("Property changes on:") { "magenta" }
           else if line.starts_with('+') { "green" }
           else if line.starts_with("@@") { "gray" }
           else if line.starts_with('-') { "red" }
           else { "white" };

    println!("{}", line.color(color));
}
