include .env

build-all: build-windows build-linux

build-windows: build-front
	TAURI_SIGNING_PRIVATE_KEY=~/.tauri/sendingUnicorns.key npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc

build-linux: build-front
	TAURI_SIGNING_PRIVATE_KEY=~/.tauri/sendingUnicorns.key bun tauri build

build-front:
	bun run build