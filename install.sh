#!/bin/sh
# Commitchi installer for Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/1pizzaslice/commitchi/main/install.sh | sh
#
# Downloads the latest prebuilt static binary from GitHub Releases and installs
# it to ~/.local/bin (override with PREFIX=/usr/local/bin, or VERSION=v0.1.0).
set -eu

REPO="1pizzaslice/commitchi"
BIN="commitchi"

say() { printf '%s\n' "$*"; }
err() { printf 'error: %s\n' "$*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || err "missing required command: $1"; }

need curl
need tar
need uname

[ "$(uname -s)" = "Linux" ] || err "this installer supports Linux only (found $(uname -s))."

case "$(uname -m)" in
  x86_64 | amd64) target="x86_64-unknown-linux-musl" ;;
  aarch64 | arm64) target="aarch64-unknown-linux-musl" ;;
  *) err "unsupported architecture: $(uname -m)" ;;
esac

version="${VERSION:-}"
if [ -z "$version" ]; then
  say "Looking up the latest release..."
  version="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep -o '"tag_name"[ ]*:[ ]*"[^"]*"' | head -n1 | cut -d'"' -f4)"
  [ -n "$version" ] || err "could not determine the latest version (set VERSION=vX.Y.Z to override)."
fi

asset="$BIN-$target.tar.gz"
base="https://github.com/$REPO/releases/download/$version"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

say "Downloading $BIN $version ($target)..."
curl -fSL --proto '=https' --tlsv1.2 "$base/$asset" -o "$tmp/$asset" \
  || err "download failed: $base/$asset"

# Verify the checksum when available and sha256sum is present.
if command -v sha256sum >/dev/null 2>&1 \
  && curl -fsSL "$base/$asset.sha256" -o "$tmp/$asset.sha256" 2>/dev/null; then
  if (cd "$tmp" && sha256sum -c "$asset.sha256" >/dev/null 2>&1); then
    say "Checksum verified."
  else
    err "checksum verification failed."
  fi
fi

tar -xzf "$tmp/$asset" -C "$tmp"
binpath="$(find "$tmp" -type f -name "$BIN" | head -n1)"
[ -n "$binpath" ] || err "could not find the $BIN binary inside the archive."
chmod +x "$binpath"

bindir="${PREFIX:-$HOME/.local/bin}"
mkdir -p "$bindir"
mv "$binpath" "$bindir/$BIN"
say "Installed $BIN to $bindir/$BIN"

case ":$PATH:" in
  *":$bindir:"*) ;;
  *)
    say ""
    say "note: $bindir is not on your PATH. Add this to your shell config:"
    say "  export PATH=\"$bindir:\$PATH\""
    ;;
esac

say ""
say "Done. Run '$BIN --help' or just '$BIN' inside any Git repository."
