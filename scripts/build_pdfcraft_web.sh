#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
PDFCRAFT_WEB_DIR="${ROOT_DIR}/vendor/pdfcraft/apps/pdfcraft-web"
TARGET_DIST="${ROOT_DIR}/public/pdfcraft"

echo "=== Building PDFCraft WebAssembly Canvas Bundle ==="
echo "PDFCraft Web Dir: ${PDFCRAFT_WEB_DIR}"
echo "Output Directory: ${TARGET_DIST}"

if [ ! -d "${PDFCRAFT_WEB_DIR}" ]; then
  echo "❌ Error: Directory ${PDFCRAFT_WEB_DIR} does not exist!"
  exit 1
fi

if ! command -v trunk &> /dev/null; then
  echo "❌ Error: 'trunk' is required but not installed or not in PATH."
  exit 1
fi

mkdir -p "${TARGET_DIST}"

cd "${PDFCRAFT_WEB_DIR}"
# Build using trunk into public/pdfcraft
trunk build --release --dist "${TARGET_DIST}"

echo "=== Successfully built PDFCraft WebAssembly into ${TARGET_DIST} ==="
