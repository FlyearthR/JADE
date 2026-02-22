FROM ubuntu:24.04

# Install build dependencies
RUN apt-get update && apt-get install -y \
    gcc \
    make \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install cbindgen
RUN cargo install cbindgen

# Copy Rust API source files
WORKDIR /build
COPY Makefile /build/Makefile
COPY Cargo.toml /build/Cargo.toml
COPY c_src/ /build/c_src
COPY src/ /build/src


# Build syscalls.so
RUN make syscalls.so

# Copy the application executable and syscalls.so to root
COPY testing/simple_server /exe/simple_server
RUN cp /build/c_src/syscalls.so /exe/syscalls.so

WORKDIR /
