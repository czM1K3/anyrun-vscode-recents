use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use serde::Deserialize;
use shellexpand::tilde;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_PREFIX: &str = "";
const DEFAULT_COMMAND: &str = "code";
const DEFAULT_ICON: &str = "com.visualstudio.code";
const DEFAULT_PATH: &str = "~/.config/Code/User/workspaceStorage";
const DEFAULT_SHOW_EMPTY: bool = false;
const DEFAULT_MAX_ENTRIES: usize = 5;

#[derive(Deserialize)]
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

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: default_prefix(),
            command: default_command(),
            icon: default_icon(),
            path: default_path(),
            show_empty: default_show_empty(),
            max_entries: default_max_entries(),
        }
    }
}

struct Project {
    full: String,
    short: String,
    id: u64,
}

pub struct State {
    results: Vec<Project>,
    config: Config,
}

#[derive(Debug, Deserialize)]
struct Workspace {
    folder: Option<String>,
}

#[init]
fn init(config_dir: RString) -> State {
    let config: Config = fs::read_to_string(format!("{}/vscode.ron", config_dir))
        .ok()
        .and_then(|content| ron::from_str(&content).ok())
        .unwrap_or_else(|| {
            eprintln!("Error parsing vscode plugin config");
            Config::default()
        });

    let base_path = PathBuf::from(tilde(&config.path).into_owned());
    let mut results: Vec<Project> = Vec::new();
    let mut id: u64 = 0;
    let mut already_have: HashSet<String> = HashSet::new();

    if let Ok(entries) = fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let file_path = entry.path().join("workspace.json");

            if file_path.is_file() {
                if let Ok(contents) = fs::read_to_string(&file_path) {
                    if let Ok(parsed) = serde_json::from_str::<Workspace>(&contents) {
                        if let Some(folder_tmp) = parsed.folder {
                            let full = folder_tmp.replace("file://", "");
                            let short = Path::new(&folder_tmp)
                                .file_name()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string();

                            if already_have.insert(full.clone()) {
                                results.push(Project { full, short, id });
                                id += 1;
                            }
                        }
                    }
                }
            }
        }
    }

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
                    id: ROption::RSome(project.id),
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
    if let Some(entry) = state.results.iter().find_map(|project| {
        if project.id == selection.id.unwrap() {
            Some(project.full.clone())
        } else {
            None
        }
    }) {
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
