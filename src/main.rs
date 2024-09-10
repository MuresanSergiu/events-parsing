pub mod error;
use std::{collections::HashMap, fs::File, io::{Read, Seek, SeekFrom}, path::Path};

use chrono::NaiveDateTime;
use error::ProjectError;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Debug)]
struct ProjectEvent {
    kind: ProjectEventKind,
    timestamp: NaiveDateTime,
    message: String,
    has_disconnected: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum ProjectEventKind {
    ConnectionError,
    Unknown,
}

fn parse_and_add_to_map(s: &str, map: &mut HashMap<String, ProjectEvent>) -> Result<(), ProjectError> {
    let (kind_str, mut s) = s.split_once(' ').ok_or_else(|| ProjectError::ParseError("Failed to get first kind part".to_owned()))?;

    let mut kind = ProjectEventKind::Unknown;
    if kind_str == "CONN" || kind_str == "START" {
        let (kind_second, rest) = s.split_once(' ').ok_or_else(|| ProjectError::ParseError("Failed to get second kind part".to_owned()))?;
        s = rest;
        if kind_second == "ERR" {
            kind = ProjectEventKind::ConnectionError;
        }
    }

    let (mut ident, s) = s.split_once(' ').ok_or_else(|| ProjectError::ParseError("Failed to get ident part".to_owned()))?;
    ident = ident.split_once('/').ok_or_else(|| ProjectError::ParseError("Failed to parse ident".to_owned()))?.1;

    let (timestamp, s) = match s.split_once(' ') {
        Some(t) => t,
        None => (s, s),
    };

    if let Some(ev) = map.get_mut(ident) {
        if kind != ProjectEventKind::Unknown {
            ev.kind = ProjectEventKind::ConnectionError;
            ev.message = s[1..s.len()-1].to_owned();
        }
        if kind_str == "DISCONNECT" {
            ev.has_disconnected = true;
        }
    } else {
        let new_event = ProjectEvent {
            kind,
            timestamp: NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S").map_err(|err| ProjectError::ParseError(format!("Failed to parse timestamp '{}' with error {}", timestamp, err)))?,
            message: s[1..s.len()-1].to_owned(),
            has_disconnected: false,
        };
        map.insert(ident.to_owned(), new_event);
    }

    Ok(())
}

fn watch<P: AsRef<Path>>(path: P) -> Result<(), ProjectError> {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(tx, notify::Config::default().with_compare_contents(true))
        .map_err(|err| ProjectError::NotifyError(format!("Failed to create watcher with error {}", err)))?;

    watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)
        .map_err(|err| ProjectError::NotifyError(format!("Failed to watch file with error {}", err)))?;

    let mut events = HashMap::new();

    let mut contents = vec![];
    let mut file = File::open(path.as_ref()).map_err(|err| ProjectError::IoError(format!("Failed to open file with error {}", err)))?;
    let mut position = file.read_to_end(&mut contents).map_err(|err| ProjectError::IoError(format!("Failed to read file with error {}", err)))?;
    let mut string = String::from_utf8(contents).map_err(|err| ProjectError::IoError(format!("Failed to convert string from utf8 with error {}", err)))?;

    loop {
        let nl_pos = string.find('\n');
        if let Some(nl_pos) = nl_pos {
            let mut new_string = string.split_off(nl_pos);
            new_string.remove(0);
            if let Err(ProjectError::ParseError(err)) = parse_and_add_to_map(string.as_str(), &mut events) {
                eprintln!("Error parsing line '{}' with error '{}'. Skipping...", string, err);
            }
            string = new_string
        } else {
            break;
        }
    }

    dbg!(&events);

    println!("Listening for changes");
    for res in rx {
        dbg!(&res);
        contents = vec![];
        match res {
            Ok(event) => {
                if event.kind.is_modify() {
                    contents.truncate(0);
                    let mut file = File::open(path.as_ref()).map_err(|err| ProjectError::IoError(format!("Failed to open file with error {}", err)))?;
                    file.seek(SeekFrom::Start(position as u64)).map_err(|_| ProjectError::IoError("Failed to seek file".to_owned()))?;
                    position += file.read_to_end(&mut contents).map_err(|err| ProjectError::IoError(format!("Failed to read file with error {}", err)))?;
                    string += String::from_utf8(contents).map_err(|err| ProjectError::IoError(format!("Failed to convert string from utf8 with error {}", err)))?.as_str();
                    loop {
                        let nl_pos = string.find('\n');
                        if let Some(nl_pos) = nl_pos {
                            let mut new_string = string.split_off(nl_pos);
                            new_string.remove(0);
                            if let Err(ProjectError::ParseError(err)) = parse_and_add_to_map(string.as_str(), &mut events) {
                                eprintln!("Error parsing line '{}' with error '{}'. Skipping...", string, err);
                            }
                            string = new_string;
                        } else {
                            break;
                        }
                    }
                    dbg!(&events);
                }
            },
            Err(error) => eprintln!("inotify error: {error:?}"),
        }
    }

    Ok(())
}

// START CONN C/AB121 2024-09-07T15:22:01
fn main() -> Result<(), ProjectError> {
    watch(Path::new("test-input.log"))?;
    Ok(())
}
