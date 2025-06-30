# This file is part of KFST.
#
# (c) 2023-2025 Iikka Hauhio <iikka.hauhio@helsinki.fi> and Théo Salmenkivi-Friberg <theo.friberg@helsinki.fi>
#
# KFST is free software: you can redistribute it and/or modify it under the
# terms of the GNU Lesser General Public License as published by the Free
# Software Foundation, either version 3 of the License, or (at your option) any
# later version.
#
# KFST is distributed in the hope that it will be useful, but WITHOUT ANY
# WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
# FOR A PARTICULAR PURPOSE. See the GNU Lesser General Public License for more
# details.
#
# You should have received a copy of the GNU Lesser General Public License
# along with KFST. If not, see <https://www.gnu.org/licenses/>.

#/bin/bash
set -e

# One or the other is needed for other targets

rustup target install x86_64-unknown-linux-gnu
rustup target install aarch64-unknown-linux-gnu

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
