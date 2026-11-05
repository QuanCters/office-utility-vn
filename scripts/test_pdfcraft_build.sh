#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
PDFCRAFT_PUBLIC_DIR="${ROOT_DIR}/public/pdfcraft"

echo "[TEST] Checking PDFCraft build output in ${PDFCRAFT_PUBLIC_DIR}..."

if [ ! -d "${PDFCRAFT_PUBLIC_DIR}" ]; then
  echo "❌ Error: Directory ${PDFCRAFT_PUBLIC_DIR} does not exist!"
  exit 1
fi

if [ ! -s "${PDFCRAFT_PUBLIC_DIR}/index.html" ]; then
  echo "❌ Error: ${PDFCRAFT_PUBLIC_DIR}/index.html is missing or empty!"
  exit 1
fi
echo "✅ index.html found"

WASM_COUNT=$(find "${PDFCRAFT_PUBLIC_DIR}" -maxdepth 1 -name "*.wasm" | wc -l)
if [ "${WASM_COUNT}" -eq 0 ]; then
  echo "❌ Error: No .wasm file found in ${PDFCRAFT_PUBLIC_DIR}!"
  exit 1
fi
echo "✅ .wasm file(s) found: ${WASM_COUNT}"

JS_COUNT=$(find "${PDFCRAFT_PUBLIC_DIR}" -maxdepth 1 -name "*.js" | wc -l)
if [ "${JS_COUNT}" -eq 0 ]; then
  echo "❌ Error: No .js file found in ${PDFCRAFT_PUBLIC_DIR}!"
  exit 1
fi
echo "✅ .js file(s) found: ${JS_COUNT}"

echo "🎉 PDFCraft build artifacts verified successfully!"
exit 0
