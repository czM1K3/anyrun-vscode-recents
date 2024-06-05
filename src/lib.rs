use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use serde::Deserialize;
use shellexpand::tilde;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "prefix")]
    prefix: String,
    #[serde(default = "command")]
    command: String,
    #[serde(default = "icon")]
    icon: String,
    #[serde(default = "path")]
    path: String,
    #[serde(default = "show_empty")]
    show_empty: bool,
    #[serde(default = "max_entries")]
    max_entries: usize,
}

fn prefix() -> String {
    "".into()
}

fn command() -> String {
    "code".into()
}

fn icon() -> String {
    "com.visualstudio.code".into()
}

fn path() -> String {
    "~/.config/Code/User/workspaceStorage".into()
}

fn show_empty() -> bool {
    false
}

fn max_entries() -> usize {
    5
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: prefix(),
            command: command(),
            icon: icon(),
            path: path(),
            show_empty: show_empty(),
            max_entries: max_entries(),
        }
    }
}

pub struct State {
    results: Vec<(String, String, u64)>,
    config: Config,
}

#[derive(Debug, Deserialize)]
struct Workspace {
    folder: Option<String>,
}

#[init]
fn init(config_dir: RString) -> State {
    let config: Config = match fs::read_to_string(format!("{}/vscode.ron", config_dir)) {
        Ok(content) => ron::from_str(&content).unwrap_or_else(|why| {
            eprintln!("Error parsing applications plugin config: {}", why);
            Config::default()
        }),
        Err(why) => {
            eprintln!("Error reading applications plugin config: {}", why);
            Config::default()
        }
    };

    let base_path_str = &(config.path.to_owned())[..];

    let expanded_path = tilde(base_path_str);
    let base_path = PathBuf::from(expanded_path.into_owned());

    let mut vec: Vec<(String, String, u64)> = Vec::new();
    let mut index: u64 = 0;

    let mut already_have: HashSet<String> = HashSet::new();

    if let Ok(entries) = fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let file_path = entry.path().join("workspace.json");

            if file_path.exists() && file_path.is_file() {
                if let Ok(contents) = fs::read_to_string(&file_path) {
                    if let Ok(parsed) = serde_json::from_str::<Workspace>(&contents) {
                        if let Some(folder_tmp) = parsed.folder {
                            let folder = Path::new(&folder_tmp);

                            let full_path = folder_tmp.replace("file://", "");
                            let shortcut =
                                folder.file_name().unwrap().to_str().unwrap().to_string();

                            if !already_have.contains(&full_path) {
                                already_have.insert(full_path.clone());
                                vec.push((full_path, shortcut, index));
                                index += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    State {
        results: vec,
        config,
    }
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "VSCode Recents".into(),
        icon: "com.visualstudio.code".into(), // Icon from the icon theme
    }
}

#[get_matches]
fn get_matches(input: RString, state: &State) -> RVec<Match> {
    if !input.starts_with(&state.config.prefix) {
        return RVec::new();
    }

    let query = input.trim_start_matches(&state.config.prefix).trim();

    if query.is_empty() {
        if !state.config.show_empty {
            return RVec::new();
        } else {
            // TODO refactor with extracting common parts
            return state
                .results
                .iter()
                .map(|(full, short, id)| Match {
                    title: format!("VSCode: {}", short).into(),
                    icon: ROption::RSome((state.config.icon.to_owned())[..].into()),
                    use_pango: false,
                    description: ROption::RSome(full[..].into()),
                    id: ROption::RSome(*id),
                })
                .take(state.config.max_entries)
                .collect();
        }
    }

    let vec = state
        .results
        .iter()
        .filter_map(|(full, short, id)| {
            if short.contains(&query.to_string()) {
                Some(Match {
                    title: format!("VSCode: {}", short).into(),
                    icon: ROption::RSome((state.config.icon.to_owned())[..].into()),
                    use_pango: false,
                    description: ROption::RSome(full[..].into()),
                    id: ROption::RSome(*id),
                })
            } else {
                None
            }
        })
        .take(state.config.max_entries)
        .collect::<RVec<Match>>();
    vec
}

#[handler]
fn handler(selection: Match, state: &State) -> HandleResult {
    let entry = state
        .results
        .iter()
        .find_map(|(full, _short, id)| {
            if *id == selection.id.unwrap() {
                Some(full)
            } else {
                None
            }
        })
        .unwrap();
    if Command::new("bash")
        .arg("-c")
        .arg(format!("{} {}", state.config.command.to_owned(), entry))
        .spawn()
        .is_err()
    {
        eprintln!("Error running vscode");
    }
    HandleResult::Close
}
