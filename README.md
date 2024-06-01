# Anyrun - VSCode Recents
Plugin for anyrun to show recently opened projects with VSCode.

## Configuration
### For VSCode
```
// <Anyrun config dir>/vscode.ron
Config(
  prefix: Some(":code"), // "" by default
  command: Some("code"),
  icon: Some("com.visualstudio.code"),
  path: Some("~/.config/Code/User/workspaceStorage"),
)
```
### For Codium
```
// <Anyrun config dir>/vscode.ron
Config(
  prefix: Some(":code"), // "" by default
  command: Some("codium"),
  icon: Some("vscodium"),
  path: Some("~/.config/VSCodium/User/workspaceStorage"),
)
```

## Building
```bash
cargo build --release && cp target/release/libvscode_recents.so ~/.config/anyrun/plugins/
```