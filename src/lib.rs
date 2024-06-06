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
    prefix: Option<String>,
    command: Option<String>,
    icon: Option<String>,
    path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prefix: Some("".into()),
            command: Some("code".to_string()),
            icon: Some("com.visualstudio.code".to_string()),
            path: Some("~/.config/Code/User/workspaceStorage".to_string()),
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
    let mut config: Config = match fs::read_to_string(format!("{}/vscode.ron", config_dir)) {
        Ok(content) => ron::from_str(&content).unwrap_or_else(|why| {
            eprintln!("Error parsing vscode plugin config: {}", why);
            Config::default()
        }),
        Err(why) => {
            eprintln!("Error reading vscode plugin config: {}", why);
            Config::default()
        }
    };

    if config.prefix.is_none() {
        config.prefix = Config::default().prefix;
    }
    if config.command.is_none() {
        config.command = Config::default().command;
    }
    if config.icon.is_none() {
        config.icon = Config::default().icon;
    }
    if config.path.is_none() {
        config.path = Config::default().path;
    }

    let base_path_str = &(config.path.to_owned().unwrap())[..];

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
    if let Some(prefix) = &state.config.prefix {
        if !input.starts_with(prefix) {
            return RVec::new();
        }
    }

    let query = input
        .trim_start_matches(
            &<std::option::Option<std::string::String> as Clone>::clone(&state.config.prefix)
                .unwrap(),
        )
        .trim();

    if query.is_empty() {
        return RVec::new();
    }

    let vec = state
        .results
        .iter()
        .filter_map(|(full, short, id)| {
            if short.contains(&query.to_string()) {
                Some(Match {
                    title: format!("VSCode: {}", short).into(),
                    icon: ROption::RSome((state.config.icon.to_owned().unwrap())[..].into()),
                    use_pango: false,
                    description: ROption::RSome(full[..].into()),
                    id: ROption::RSome(*id),
                })
            } else {
                None
            }
        })
        .take(5)
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
        .arg(format!(
            "{} {}",
            state.config.command.to_owned().unwrap(),
            entry
        ))
        .spawn()
        .is_err()
    {
        eprintln!("Error running vscode");
    }
    HandleResult::Close
}
