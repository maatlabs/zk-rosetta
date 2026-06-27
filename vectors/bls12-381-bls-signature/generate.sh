#!/usr/bin/env bash
# Regenerates vector.toml for the bls12-381-bls-signature vector.
#
# Builds the committed generate.rs in a throwaway Cargo project with pinned,
# audited dependencies and prints the normalized vector to stdout. The generator
# is deterministic, so the output matches the committed vector.toml byte for byte.
#
#   ./generate.sh > vector.toml
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

mkdir -p "$work/src"
cp "$here/generate.rs" "$work/src/main.rs"

cat > "$work/Cargo.toml" <<'TOML'
[package]
name = "bls-vector-generator"
version = "0.0.0"
edition = "2021"

[dependencies]
bls-signatures = { version = "=0.15.0", default-features = false, features = ["blst"] }
blstrs = "=0.7.1"
group = "0.13"
hex = "0.4"
TOML

cargo run --quiet --release --manifest-path "$work/Cargo.toml"
