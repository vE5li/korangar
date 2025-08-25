# Requirements

### 🦀 Cargo
Cargo (the Rust package manager) can best be installed and managed through [Rustup](https://rustup.rs/).

Note that Korangar requires Rust nightly to compile. If you need help configuring Rustup accordingly, please read [this stackoverflow issue](https://stackoverflow.com/questions/58226545/how-to-switch-between-rust-toolchains).

# OS-specific

### 🪟 Notes on Windows
On Windows you will need the following additional dependencies to be installed:
- CMake
- Ninja
- Git
- Python3

It is recommended installing these through a package manager like `choco` or `scoop`, since it saves a lot of manual configuration.

e.g.:
```powershell
choco install ninja python3 git cmake --installargs 'ADD_CMAKE_TO_PATH=System'
```

### ❄️ Nix & NixOS

There is a `flake.nix` in the repository that exposes a dev shell with all dependencies for testing and running Korangar on Linux and MacOS.

# Compiling
You can compile Korangar by running:

```fish
cargo build --release
```

### 🪲 Debug tools

To gain access to Korangar's built-in developer tools, explicitly enable the `debug` feature:

```fish
cargo build --release --features debug
```

If your terminal supports Unicode, you might want to enable the `unicode` feature as well:

```fish
cargo build --release --features "debug unicode"
```

# Running

For information on how to use Korangar, please take a look at the wiki page called [Running](Running.md).
