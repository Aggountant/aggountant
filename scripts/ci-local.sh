#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly ROOT
readonly MSRV="1.89.0"
readonly CARGO_DENY_VERSION="0.20.2"
readonly CARGO_DENY_BASE_URL="https://github.com/EmbarkStudios/cargo-deny/releases/download/${CARGO_DENY_VERSION}"

cd "$ROOT"

for toolchain in stable "$MSRV"; do
  if ! rustup run "$toolchain" rustc --version >/dev/null 2>&1; then
    printf 'error: Rust toolchain %s is unavailable; install it with: rustup toolchain install %s\n' \
      "$toolchain" "$toolchain" >&2
    exit 1
  fi
done

run_cargo_deny() {
  if cargo deny --version >/dev/null 2>&1; then
    cargo deny check advisories bans licenses sources
    return
  fi

  (
  local target checksum archive temp_dir deny_bin actual_checksum
  case "$(uname -s)-$(uname -m)" in
    Darwin-arm64)
      target="aarch64-apple-darwin"
      checksum="fe67d82a10d8597a3549364cb733a3f9cc1bfff9031b7ae46384a9f2a72090c3"
      ;;
    Darwin-x86_64)
      target="x86_64-apple-darwin"
      checksum="248da7f581724e470071990c088ffc55c811981715f4cbdb258621fb79f8b7a6"
      ;;
    Linux-aarch64 | Linux-arm64)
      target="aarch64-unknown-linux-musl"
      checksum="995c82be0defc7a025cae49a2aa2644ce8245c9a3318fc4103907c6a285e8c7d"
      ;;
    Linux-x86_64)
      target="x86_64-unknown-linux-musl"
      checksum="9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f"
      ;;
    *)
      printf 'error: cargo-deny is not installed and no prebuilt binary is configured for %s/%s\n' \
        "$(uname -s)" "$(uname -m)" >&2
      exit 1
      ;;
  esac

  temp_dir="$(mktemp -d)"
  archive="$temp_dir/cargo-deny.tar.gz"
  trap 'rm -rf "$temp_dir"' EXIT

  curl --fail --silent --show-error --location \
    "$CARGO_DENY_BASE_URL/cargo-deny-${CARGO_DENY_VERSION}-${target}.tar.gz" \
    --output "$archive"

  if command -v shasum >/dev/null 2>&1; then
    actual_checksum="$(shasum -a 256 "$archive" | awk '{print $1}')"
  elif command -v sha256sum >/dev/null 2>&1; then
    actual_checksum="$(sha256sum "$archive" | awk '{print $1}')"
  else
    printf 'error: shasum or sha256sum is required to verify cargo-deny\n' >&2
    exit 1
  fi

  if [[ "$actual_checksum" != "$checksum" ]]; then
    printf 'error: cargo-deny %s checksum mismatch\n' "$CARGO_DENY_VERSION" >&2
    exit 1
  fi

  tar -xzf "$archive" -C "$temp_dir"
  deny_bin="$(find "$temp_dir" -type f -name cargo-deny -perm -u+x -print -quit)"
  if [[ -z "$deny_bin" ]]; then
    printf 'error: cargo-deny executable missing from release archive\n' >&2
    exit 1
  fi
  "$deny_bin" check advisories bans licenses sources
  )
}

printf '\n== stable fmt ==\n'
rustup run stable cargo fmt --all --check

printf '\n== stable clippy ==\n'
rustup run stable cargo clippy --workspace --all-targets -- -D warnings

printf '\n== cargo-deny ==\n'
run_cargo_deny

for toolchain in stable "$MSRV"; do
  printf '\n== %s build ==\n' "$toolchain"
  rustup run "$toolchain" cargo build --workspace --locked

  printf '\n== %s test ==\n' "$toolchain"
  rustup run "$toolchain" cargo test --workspace --locked
done

printf '\n== user guide ==\n'
rustup run stable scripts/check-user-guide.sh

printf '\nAll local CI gates passed.\n'
