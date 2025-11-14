#!/usr/bin/env bash

set -euo pipefail

TARGET_TRIPLE=${TARGET_TRIPLE:-x86_64-pc-windows-gnu}
BUILD_PROFILE=${BUILD_PROFILE:-release}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_ROOT="${REPO_ROOT}/dist"

cd "${REPO_ROOT}"

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required to build the project." >&2
    exit 1
fi

if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
    echo "mingw-w64 (x86_64-w64-mingw32-gcc) is required for Windows builds." >&2
    echo "Install mingw-w64 (e.g., 'sudo apt install mingw-w64')." >&2
    exit 1
fi

VERSION=${VERSION:-$(
    cargo metadata --no-deps --format-version 1 |
        python3 -c 'import json,sys; meta=json.load(sys.stdin); print(next(pkg["version"] for pkg in meta["packages"] if pkg["name"]=="bestbefors"))'
)}

ARTIFACT_DIR="${DIST_ROOT}/bestbefors-${VERSION}-windows-amd64"
ZIP_PATH="${ARTIFACT_DIR}.zip"
TARGET_DIR="${BUILD_PROFILE}"
if [[ "${BUILD_PROFILE}" == "release" ]]; then
    TARGET_DIR="release"
fi
TARGET_BIN="target/${TARGET_TRIPLE}/${TARGET_DIR}/bestbefors-cli.exe"

echo "Building ${TARGET_TRIPLE} (${BUILD_PROFILE})…"
if [[ "${BUILD_PROFILE}" == "release" ]]; then
    cargo build --target "${TARGET_TRIPLE}" --release
else
    cargo build --target "${TARGET_TRIPLE}" --profile "${BUILD_PROFILE}"
fi

if [[ ! -f "${TARGET_BIN}" ]]; then
    echo "Expected binary not found at ${TARGET_BIN}" >&2
    exit 1
fi

rm -rf "${ARTIFACT_DIR}"
mkdir -p "${ARTIFACT_DIR}"

cp "${TARGET_BIN}" "${ARTIFACT_DIR}/"

for path in assets config migration initial_db.bash README.md LICENSE-Apache.txt LICENSE-MIT.txt src/fixtures scripts/start_server.ps1; do
    if [[ -e "${path}" ]]; then
        cp -R "${path}" "${ARTIFACT_DIR}/"
    fi
done

(
    cd "${DIST_ROOT}"
    rm -f "$(basename "${ZIP_PATH}")"
    zip -r "$(basename "${ZIP_PATH}")" "$(basename "${ARTIFACT_DIR}")" >/dev/null
)

echo "Created $(basename "${ZIP_PATH}") in ${DIST_ROOT}"
