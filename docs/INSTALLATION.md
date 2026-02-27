# Harness Installation Guide

## 1. Supported setup

`harness` is a Rust CLI distributed as a single binary.

Required:
- Rust stable toolchain (`rustc`, `cargo`)
- Git
- POSIX shell (`bash`) for helper script usage

Optional but recommended:
- `~/.local/bin` on `PATH`

## 2. Install from local clone (recommended for contributors)

```bash
git clone git@github.com:mudrii/harness.git
cd harness
./scripts/install.sh --method path --force
```

Default install root is `~/.local`, so the binary path becomes:

```text
~/.local/bin/harness
```

## 3. Install directly from GitHub

```bash
./scripts/install.sh --method git --repo-url https://github.com/mudrii/harness --force
```

This uses Cargo git installation and does not require a local `Cargo.toml`.

## 4. Script options

`scripts/install.sh` options:

- `--method auto|path|git`
- `--repo-url <url>`
- `--crate-name <name>`
- `--install-root <path>`
- `--force`
- `--no-locked`
- `--help`

Environment override:

- `HARNESS_INSTALL_ROOT=<path>` (same effect as `--install-root`)

## 5. Manual install (without helper script)

From repository root:

```bash
cargo install --path . --locked --force
```

From remote git:

```bash
cargo install --git https://github.com/mudrii/harness harness --locked --force
```

## 6. Validate installation

```bash
harness --help
harness analyze /path/to/repo --format markdown
```

If command is not found:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Add that line to your shell profile for persistence (`~/.zshrc` or `~/.bashrc`).

## 7. Upgrade

Re-run installation with `--force`:

```bash
./scripts/install.sh --method path --force
```

or

```bash
./scripts/install.sh --method git --repo-url https://github.com/mudrii/harness --force
```

## 8. Uninstall

If installed via script default root:

```bash
rm -f ~/.local/bin/harness
```

If a custom install root was used, remove `<install-root>/bin/harness`.

## 9. Troubleshooting

### `cargo: command not found`

Install Rust toolchain from [rustup.rs](https://rustup.rs/), then retry.

### `harness` not on PATH

Add the install bin directory to `PATH` and restart the shell.

### Permission issues in install root

Use a user-writable root such as:

```bash
./scripts/install.sh --install-root "$HOME/.local" --force
```

### Build errors during install

Check toolchain and run:

```bash
rustc --version
cargo --version
```

Then run from repository root:

```bash
cargo check --all-targets
```
