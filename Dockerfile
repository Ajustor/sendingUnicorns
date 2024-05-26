FROM ivangabriele/tauri:debian-bullseye-18 AS base

RUN apt update
RUN apt install libwebkit2gtk-4.0-dev build-essential curl wget libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev -y
RUN cargo install tauri-cli

RUN curl -fsSL https://bun.sh/install | bash

FROM base AS macos
RUN rustup target add x86_64-apple-darwin

FROM base AS windows
RUN apt install nsis lld llvm -y
RUN rustup target add x86_64-pc-windows-gnu
RUN cargo install xwin
RUN xwin --accept-license splat --output ~/.xwin --disable-symlinks

FROM base as linux
RUN rustup target add x86_64-unknown-linux-gnu