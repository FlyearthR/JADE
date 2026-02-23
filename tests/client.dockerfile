FROM ubuntu:latest

# Install build dependencies
RUN apt-get update && apt-get install -y \
    gcc \
    make \
    curl

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


# Build syscalls.so, client and server
RUN make syscalls.so
RUN make usetest || echo "skip fail"

# Copy the application executable and syscalls.so to root
COPY testing/simple_client /exe/simple_client
RUN cp /build/c_src/syscalls.so /exe/syscalls.so
RUN mkdir /logs

WORKDIR /exe
