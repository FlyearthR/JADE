ARG image

FROM ubuntu:latest

RUN apt update
RUN apt install -y wget gawk bison build-essential curl snapd python3 sudo make git cmake openssl pkg-config libssl-dev libcunit1 libcunit1-doc libcunit1-dev

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install cbindgen

RUN mkdir /network-time-simulator
WORKDIR /network-time-simulator
COPY . .

COPY <<EOF /network-time-simulator/c_src/Makefile
CC = gcc
CFLAGS = -g -DDEBUG -Wall

%.o: %.c
	$(CC) -static -static-libgcc -fPIC -I. -llibAPI -c $^ -o $@

syscalls.so: communication.o helper.o syscalls.o
	$(CC) -shared -o syscalls.so communication.o helper.o syscalls.o libAPI.a -ldl && mkdir -p ../target/syscalls && cp syscalls.so ../target/syscalls/syscalls.so

test_ffi:
	$(CC) ${CFLAGS} -I. -o tests/test_ffi tests/test_ffi.c libAPI.a -lcunit
EOF

WORKDIR /network-time-simulator

RUN make all

ARG image

FROM $image:latest

COPY --from=0 /network-time-simulator /network-time-simulator
