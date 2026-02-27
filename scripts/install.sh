#!/usr/bin/env bash
set -euo pipefail

METHOD="auto"
REPO_URL="https://github.com/mudrii/harness"
CRATE_NAME="harness"
INSTALL_ROOT="${HARNESS_INSTALL_ROOT:-$HOME/.local}"
FORCE=false
LOCKED=true

print_help() {
  cat <<'EOF'
Harness installer

Usage:
  ./scripts/install.sh [options]

Options:
  --method <auto|path|git>   Install source selection (default: auto)
  --repo-url <url>           Git repository URL for --method git
  --crate-name <name>        Crate/binary name (default: harness)
  --install-root <path>      Cargo install root (default: ~/.local)
  --force                    Reinstall even if already installed
  --no-locked                Do not pass --locked to cargo install
  --help                     Show this help

Environment:
  HARNESS_INSTALL_ROOT       Same as --install-root
EOF
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command '$1' is not available on PATH" >&2
    exit 1
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --method)
      METHOD="${2:-}"
      shift 2
      ;;
    --repo-url)
      REPO_URL="${2:-}"
      shift 2
      ;;
    --crate-name)
      CRATE_NAME="${2:-}"
      shift 2
      ;;
    --install-root)
      INSTALL_ROOT="${2:-}"
      shift 2
      ;;
    --force)
      FORCE=true
      shift
      ;;
    --no-locked)
      LOCKED=false
      shift
      ;;
    --help|-h)
      print_help
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      print_help
      exit 1
      ;;
  esac
done

if [[ "$METHOD" != "auto" && "$METHOD" != "path" && "$METHOD" != "git" ]]; then
  echo "error: --method must be one of: auto, path, git" >&2
  exit 1
fi

require_cmd cargo
mkdir -p "$INSTALL_ROOT"

CARGO_ARGS=(install)
if [[ "$LOCKED" == true ]]; then
  CARGO_ARGS+=(--locked)
fi
if [[ "$FORCE" == true ]]; then
  CARGO_ARGS+=(--force)
fi
CARGO_ARGS+=(--root "$INSTALL_ROOT")

if [[ "$METHOD" == "auto" ]]; then
  if [[ -f "Cargo.toml" ]]; then
    METHOD="path"
  else
    METHOD="git"
  fi
fi

if [[ "$METHOD" == "path" ]]; then
  if [[ ! -f "Cargo.toml" ]]; then
    echo "error: Cargo.toml not found in current directory; cannot use --method path" >&2
    exit 1
  fi
  echo "Installing '$CRATE_NAME' from local path into $INSTALL_ROOT ..."
  cargo "${CARGO_ARGS[@]}" --path .
else
  echo "Installing '$CRATE_NAME' from git repo '$REPO_URL' into $INSTALL_ROOT ..."
  cargo "${CARGO_ARGS[@]}" --git "$REPO_URL" "$CRATE_NAME"
fi

BIN_PATH="$INSTALL_ROOT/bin/$CRATE_NAME"
if [[ ! -x "$BIN_PATH" ]]; then
  echo "error: installation completed but binary not found at $BIN_PATH" >&2
  exit 1
fi

echo "Installed: $BIN_PATH"
if [[ ":$PATH:" != *":$INSTALL_ROOT/bin:"* ]]; then
  echo "warning: $INSTALL_ROOT/bin is not on PATH"
  echo "Add this to your shell profile:"
  echo "  export PATH=\"$INSTALL_ROOT/bin:\$PATH\""
fi

echo "Run '$CRATE_NAME --help' to verify."
