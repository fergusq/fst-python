#/bin/bash
set -e

# One or the other is needed for other targets

rustup target install x86_64-unknown-linux-gnu
rustup target install aarch64-unknown-linux-gnu

# Windows targets (msvc)

rustup target install x86_64-pc-windows-msvc
rustup target install aarch64-pc-windows-msvc
uvx maturin build --release --target x86_64-pc-windows-msvc --zig
uvx maturin build --release --target aarch64-pc-windows-msvc --zig
rustup target uninstall x86_64-pc-windows-msvc
rustup target uninstall aarch64-pc-windows-msvc

# Windows targets (mingw)

rustup target install x86_64-pc-windows-gnu
rustup target install aarch64-pc-windows-gnullvm
uvx maturin build --release --target x86_64-pc-windows-gnu --zig
uvx maturin build --release --target aarch64-pc-windows-gnullvm --zig
rustup target uninstall x86_64-pc-windows-gnu
rustup target uninstall aarch64-pc-windows-gnullvm

# Apple targets

rustup target install x86_64-apple-darwin
rustup target install aarch64-apple-darwin
uvx maturin build --release --target x86_64-apple-darwin --zig
uvx maturin build --release --target aarch64-apple-darwin --zig
rustup target uninstall aarch64-apple-darwin
rustup target uninstall x86_64-apple-darwin

# Linux targets (musl)

rustup target install aarch64-unknown-linux-musl
rustup target install x86_64-unknown-linux-musl
rustup target install i686-unknown-linux-musl
uvx maturin build --release --target aarch64-unknown-linux-musl --zig
uvx maturin build --release --target x86_64-unknown-linux-musl --zig
uvx maturin build --release --target i686-unknown-linux-musl --zig
rustup target uninstall aarch64-unknown-linux-musl
rustup target uninstall x86_64-unknown-linux-musl
rustup target uninstall i686-unknown-linux-musl

# Linux targets (gnu)

rustup target install i686-unknown-linux-gnu
uvx maturin build --release --target aarch64-unknown-linux-gnu --zig
uvx maturin build --release --target x86_64-unknown-linux-gnu --zig
uvx maturin build --release --target i686-unknown-linux-gnu --zig
rustup target uninstall i686-unknown-linux-gnu
