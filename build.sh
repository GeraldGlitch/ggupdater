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
	local llvm_mingw="$HOME/.local/share/llvm-mingw"

	if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
		if [ -x "$llvm_mingw/bin/x86_64-w64-mingw32-gcc" ]; then
			export PATH="$llvm_mingw/bin:$PATH"
		else
			echo "Falta mingw-w64. Instala 'mingw-w64-gcc' o deja llvm-mingw en $llvm_mingw." >&2
			exit 1
		fi
	fi

	# llvm-mingw no incluye libgcc; Rust pide -lgcc/-lgcc_eh. Se enlazan a compiler-rt/libunwind.
	local libdir="$llvm_mingw/x86_64-w64-mingw32/lib"
	if [ -d "$libdir" ] && [ ! -e "$libdir/libgcc.a" ]; then
		local builtins
		builtins=$(ls "$llvm_mingw"/lib/clang/*/lib/windows/libclang_rt.builtins-x86_64.a 2>/dev/null | head -n1)
		if [ -n "$builtins" ] && [ -f "$libdir/libunwind.a" ]; then
			ln -sf "$builtins" "$libdir/libgcc.a"
			ln -sf "$libdir/libunwind.a" "$libdir/libgcc_eh.a"
		fi
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
