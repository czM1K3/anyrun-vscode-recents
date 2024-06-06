# Anyrun - VSCode Recents
Plugin for anyrun to show recently opened projects with VSCode.

## Configuration
### For VSCode
```rust
// <Anyrun config dir>/vscode.ron
Config(
  prefix: ":code", // "" by default
  command: "code",
  icon: "com.visualstudio.code",
  path: "~/.config/Code/User/workspaceStorage",
  show_empty: false,
  max_entries: 5
)
```
### For Codium
```rust
// <Anyrun config dir>/vscode.ron
Config(
  prefix: ":code", // "" by default
  command: "codium",
  icon: "vscodium",
  path: "~/.config/VSCodium/User/workspaceStorage",
  show_empty: false,
  max_entries: 5
)
```

## Building
```bash
cargo build --release && cp target/release/libvscode_recents.so ~/.config/anyrun/plugins/
```
