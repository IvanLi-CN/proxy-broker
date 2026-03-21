#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
usage: package_release_asset.sh --platform <linux|darwin> --arch <amd64|arm64> --output-dir <path> [--repo-root <path>]

Build the embedded web UI, compile the release binary, and package a GitHub
Release tarball plus per-asset SHA256 metadata.

Required environment:
  APP_EFFECTIVE_VERSION   Version/tag string to embed into the binary and asset name
Optional environment:
  RELEASE_ASSET_TAG       Override the version segment used in the tarball name
EOF
}

platform=""
arch=""
output_dir=""
repo_root=""

while (($# > 0)); do
  case "$1" in
    --platform)
      platform="${2:-}"
      shift 2
      ;;
    --arch)
      arch="${2:-}"
      shift 2
      ;;
    --output-dir)
      output_dir="${2:-}"
      shift 2
      ;;
    --repo-root)
      repo_root="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "${platform}" || -z "${arch}" || -z "${output_dir}" ]]; then
  usage >&2
  exit 1
fi

if [[ -z "${APP_EFFECTIVE_VERSION:-}" ]]; then
  echo "APP_EFFECTIVE_VERSION is required" >&2
  exit 1
fi

release_asset_tag="${RELEASE_ASSET_TAG:-${APP_EFFECTIVE_VERSION}}"

case "${platform}" in
  linux|darwin) ;;
  *)
    echo "unsupported platform: ${platform}" >&2
    exit 1
    ;;
esac

case "${arch}" in
  amd64|arm64) ;;
  *)
    echo "unsupported arch: ${arch}" >&2
    exit 1
    ;;
esac

for cmd in bun cargo tar install; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "missing required command: ${cmd}" >&2
    exit 1
  fi
done

detect_host_target() {
  local host_os
  local host_cpu

  host_os="$(uname -s)"
  host_cpu="$(uname -m)"

  case "${host_os}" in
    Linux) host_os="linux" ;;
    Darwin) host_os="darwin" ;;
    *)
      echo "unsupported host platform: ${host_os}" >&2
      exit 1
      ;;
  esac

  case "${host_cpu}" in
    x86_64) host_cpu="amd64" ;;
    arm64|aarch64) host_cpu="arm64" ;;
    *)
      echo "unsupported host architecture: ${host_cpu}" >&2
      exit 1
      ;;
  esac

  printf '%s %s\n' "${host_os}" "${host_cpu}"
}

hash_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
    return
  fi

  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
    return
  fi

  echo "missing sha256 checksum tool" >&2
  exit 1
}

if [[ -n "${repo_root}" ]]; then
  repo_root="$(cd "${repo_root}" && pwd)"
else
  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fi

if [[ ! -f "${repo_root}/Cargo.toml" || ! -d "${repo_root}/web" ]]; then
  echo "repo root does not look like proxy-broker: ${repo_root}" >&2
  exit 1
fi

binary_name="proxy-broker"
asset_stem="${binary_name}-${release_asset_tag}-${platform}-${arch}"
read -r host_platform host_arch <<<"$(detect_host_target)"

if [[ "${platform}" != "${host_platform}" || "${arch}" != "${host_arch}" ]]; then
  echo "requested ${platform}/${arch} but host runner is ${host_platform}/${host_arch}" >&2
  exit 1
fi

mkdir -p "${output_dir}"
output_dir="$(cd "${output_dir}" && pwd)"

asset_dir="${output_dir}/${asset_stem}"
asset_path="${output_dir}/${asset_stem}.tar.gz"
checksum_path="${output_dir}/${asset_stem}.sha256"

rm -rf "${asset_dir}" "${asset_path}" "${checksum_path}"

(
  cd "${repo_root}/web"
  bun run build
)

(
  cd "${repo_root}"
  APP_EFFECTIVE_VERSION="${APP_EFFECTIVE_VERSION}" cargo build --locked --release
)

mkdir -p "${asset_dir}"
install -m 755 "${repo_root}/target/release/${binary_name}" "${asset_dir}/${binary_name}"

tar -C "${output_dir}" -czf "${asset_path}" "${asset_stem}"

checksum="$(hash_file "${asset_path}")"
printf '%s  %s\n' "${checksum}" "$(basename "${asset_path}")" > "${checksum_path}"

archive_listing="$(tar -tzf "${asset_path}")"
printf '%s\n' "${archive_listing}" | grep -Fx "${asset_stem}/" >/dev/null
printf '%s\n' "${archive_listing}" | grep -Fx "${asset_stem}/${binary_name}" >/dev/null
grep -Fx "${checksum}  $(basename "${asset_path}")" "${checksum_path}" >/dev/null

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  {
    echo "asset_stem=${asset_stem}"
    echo "asset_path=${asset_path}"
    echo "checksum_path=${checksum_path}"
  } >> "${GITHUB_OUTPUT}"
fi

echo "packaged ${asset_path}"
