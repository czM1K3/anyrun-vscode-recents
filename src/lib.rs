use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use rusqlite::{Connection, Error};
use serde::Deserialize;
use shellexpand::tilde;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_PREFIX: &str = "";
const DEFAULT_COMMAND: &str = "code";
const DEFAULT_ICON: &str = "com.visualstudio.code";
const DEFAULT_PATH: &str = "~/.config/Code";
const DEFAULT_SHOW_EMPTY: bool = false;
const DEFAULT_MAX_ENTRIES: usize = 5;

const DB_QUERY: &str =
    "SELECT [value] FROM ItemTable WHERE [key] = 'history.recentlyOpenedPathsList'";

#[derive(Deserialize, Default)]
pub struct Config {
    #[serde(default = "default_prefix")]
    prefix: String,
    #[serde(default = "default_command")]
    command: String,
    #[serde(default = "default_icon")]
    icon: String,
    #[serde(default = "default_path")]
    path: String,
    #[serde(default = "default_show_empty")]
    show_empty: bool,
    #[serde(default = "default_max_entries")]
    max_entries: usize,
}

fn default_prefix() -> String {
    DEFAULT_PREFIX.into()
}
fn default_command() -> String {
    DEFAULT_COMMAND.into()
}
fn default_icon() -> String {
    DEFAULT_ICON.into()
}
fn default_path() -> String {
    DEFAULT_PATH.into()
}
fn default_show_empty() -> bool {
    DEFAULT_SHOW_EMPTY
}
fn default_max_entries() -> usize {
    DEFAULT_MAX_ENTRIES
}

struct Project {
    full: String,
    short: String,
    id: usize,
}

pub struct State {
    results: Vec<Project>,
    config: Config,
}

fn get_folders(config: &Config) -> Result<Vec<String>, Error> {
    let db_path =
        PathBuf::from(tilde(&config.path).into_owned()).join("User/globalStorage/state.vscdb");
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(DB_QUERY)?;
    let value: String = stmt.query_row([], |row| row.get(0))?;

    let recently_opened_paths_list = serde_json::from_str::<serde_json::Value>(&value)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    let binding = &vec![];
    let entries = recently_opened_paths_list["entries"]
        .as_array()
        .unwrap_or_else(|| {
            eprintln!("Error parsing entries as array");
            binding
        });

    Ok(entries
        .iter()
        .filter_map(|entry| entry["folderUri"].as_str())
        .filter(|item| item.starts_with("file://"))
        .map(|item| item.trim_start_matches("file://").to_owned())
        .collect())
}

#[init]
fn init(config_dir: RString) -> State {
    let config_path = format!("{}/vscode.ron", config_dir);
    let config: Config = match fs::read_to_string(&config_path) {
        Ok(content) => match ron::from_str(&content) {
            Ok(cfg) => cfg,
            Err(err) => {
                eprintln!("Error parsing config file {}: {}", config_path, err);
                Config::default()
            }
        },
        Err(err) => {
            eprintln!("Error reading config file {}: {}", config_path, err);
            Config::default()
        }
    };

    let results = match get_folders(&config) {
        Ok(folders) => folders
            .into_iter()
            .enumerate()
            .map(|(id, full)| Project {
                short: Path::new(&full)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .to_string(),
                full,
                id,
            })
            .collect(),
        Err(err) => {
            eprintln!("Error getting folders: {}", err);
            Vec::new()
        }
    };

    State { results, config }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "VSCode Recents".into(),
        icon: DEFAULT_ICON.into(),
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    if !input.starts_with(&state.config.prefix) {
        return RVec::new();
    }

    let query = input.trim_start_matches(&state.config.prefix).trim();

    if query.is_empty() && !state.config.show_empty {
        return RVec::new();
    }

    state
        .results
        .iter()
        .filter_map(|project| {
            if query.is_empty() || project.short.contains(query) {
                Some(Match {
                    title: format!("VSCode: {}", project.short).into(),
                    icon: ROption::RSome(state.config.icon.clone().into()),
                    use_pango: false,
                    description: ROption::RSome(project.full.clone().into()),
                    id: ROption::RSome(project.id as u64),
                })
            } else {
                None
            }
        })
        .take(state.config.max_entries)
        .collect()
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    if let Some(entry) = state
        .results
        .iter()
        .find(|project| project.id as u64 == selection.id.unwrap())
        .map(|project| project.full.clone())
    {
        if Command::new("bash")
            .arg("-c")
            .arg(format!("{} {}", state.config.command, entry))
            .spawn()
            .is_err()
        {
            eprintln!("Error running VSCode");
        }
    }

    HandleResult::Close
}
