FROM rust:bookworm AS base

RUN apt update
RUN apt install libwebkit2gtk-4.1-dev build-essential curl wget file libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev libxdo-dev -y
RUN cargo install tauri-cli@=2.1.0

RUN curl -fsSL https://bun.sh/install | bash

FROM base as macos
# FROM briceburg/macos:build AS macos
# RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
# RUN echo 'source $HOME/.cargo/env' >> $HOME/.bashrc
# RUN source $HOME/.bashrc
# RUN cargo install tauri-cli
RUN rustup target add x86_64-apple-darwin

FROM base AS windows
RUN apt install nsis lld llvm gcc-mingw-w64 clang -y
RUN rustup target add x86_64-pc-windows-msvc x86_64-pc-windows-gnu 
RUN rustup update
RUN cargo install cargo-xwin

FROM base as linux
RUN rustup target add x86_64-unknown-linux-gnu
