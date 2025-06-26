#/bin/bash
set -e
uvx maturin build --release --target aarch64-apple-darwin --zig
uvx maturin build --release --target aarch64-unknown-linux-musl --zig
uvx maturin build --release --target aarch64-unknown-linux-gnu --zig
uvx maturin build --release --target x86_64-apple-darwin --zig
uvx maturin build --release --target x86_64-unknown-linux-musl --zig
uvx maturin build --release --target x86_64-unknown-linux-gnu --zig
