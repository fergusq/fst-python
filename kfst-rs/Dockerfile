FROM fedora:latest
RUN dnf install -y gcc python3 patchelf zig mingw64-gcc-c++ lld-link
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
ENV PATH="/root/.local/bin/:$PATH"
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh; chmod +x rustup.sh; ./rustup.sh -y; rm rustup.sh
ENV PATH="/root/.cargo/bin:$PATH"
