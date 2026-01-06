# BorrowChecker

Share expenses with friends to keep track of who owes who. Data of an expense sharing group is stored in a user managed git repository. `BorrowChecker` is a desktop/mobile app built with Tauri.

## Development

### Dependencies
- Tauri CLI: `cargo install tauri-cli --version "^2.0" --locked`
- Tauri dependencies: `cargo tauri info` check 'Environment' section
### Running
```bash
cargo tauri dev
```

### Building
```bash
cargo tauri build
```
#### Android Build
Requirements
- Android Studio installed
- Android Studio configured (NDK and tools installed) 

Create Android Studio project via Tauri
```bash
cargo tauri android init
```

Build Android debug build
```bash
cargo tauri android dev
```
Run or debug project via Android Studio on the target of choice.
