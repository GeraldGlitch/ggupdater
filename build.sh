#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

usage() {
	echo "uso: ./build.sh [native|windows]" >&2
	exit 1
}

build_native() {
	cargo build --release
	mkdir -p dist
	cp target/release/ggupdater dist/ggupdater.x86_64
	chmod +x dist/ggupdater.x86_64
	echo "OK -> dist/ggupdater.x86_64"
}

build_windows() {
	if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
		echo "Falta mingw-w64 (x86_64-w64-mingw32-gcc). Instálalo y reintenta." >&2
		exit 1
	fi
	rustup target add x86_64-pc-windows-gnu
	cargo build --release --target x86_64-pc-windows-gnu
	mkdir -p dist
	cp target/x86_64-pc-windows-gnu/release/ggupdater.exe dist/ggupdater.exe
	echo "OK -> dist/ggupdater.exe"
}

case "${1:-native}" in
native) build_native ;;
windows) build_windows ;;
*) usage ;;
esac
