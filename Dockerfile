FROM ivangabriele/tauri:debian-bullseye-18 AS base

RUN apt update
RUN apt install libwebkit2gtk-4.0-dev build-essential curl wget libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev -y
RUN cargo install tauri-cli

RUN curl -fsSL https://bun.sh/install | bash

FROM base as macos
# FROM sickcodes/docker-osx AS macos
# RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
# RUN echo 'source $HOME/.cargo/env' >> $HOME/.bashrc
# RUN source $HOME/.bashrc
# RUN cargo install tauri-cli
RUN rustup target add x86_64-apple-darwin

FROM base AS windows
RUN apt install nsis lld llvm -y
RUN rustup target add x86_64-pc-windows-msvc
RUN cargo install xwin
RUN xwin --accept-license splat --output ~/.xwin --disable-symlinks

FROM base as linux
RUN rustup target add x86_64-unknown-linux-gnu