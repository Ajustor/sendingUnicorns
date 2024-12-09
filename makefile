build-all: build-windows build-linux

build-windows: build-front
	npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc

build-linux: build-front
	bun tauri build

build-front:
	bun run build