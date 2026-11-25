#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
GRIDCRAFT_WEB_DIR="${ROOT_DIR}/vendor/gridcraft/apps/gridcraft-web"
TARGET_DIST="${ROOT_DIR}/public/gridcraft"

echo "=== Building GridCraft WebAssembly Canvas Bundle ==="
echo "GridCraft Web Dir: ${GRIDCRAFT_WEB_DIR}"
echo "Output Directory: ${TARGET_DIST}"

if [ ! -d "${GRIDCRAFT_WEB_DIR}" ]; then
  echo "❌ Error: Directory ${GRIDCRAFT_WEB_DIR} does not exist!"
  exit 1
fi

if ! command -v trunk &> /dev/null; then
  echo "❌ Error: 'trunk' is required but not installed or not in PATH."
  exit 1
fi

mkdir -p "${TARGET_DIST}"

cd "${GRIDCRAFT_WEB_DIR}"
# Build using trunk into public/gridcraft
trunk build --release --dist "${TARGET_DIST}" --public-url "./"

echo "=== Successfully built GridCraft WebAssembly into ${TARGET_DIST} ==="
