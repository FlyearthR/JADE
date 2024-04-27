CC = gcc
CFLAGS = -g

.PHONY: syscalls.so all test API leader clean little_clean

test: all
	cargo test
	cp -f target/debug/leader testing/leader
	cp -f target/syscalls/syscalls.so testing/syscalls.so
	cp -f target/examples/miniP_client tests/client
	cp -f target/examples/miniP_server tests/server
	cd testing && RUST_BACKTRACE=1 ./leader

all: syscalls.so leader examples API 

syscalls.so: API
	cd c_src && $(MAKE) syscalls.so

examples: target/examples/miniP_client target/examples/miniP_server

target/examples/miniP_client:
	cd examples/miniP && $(CC) ${CFLAGS} miniP_client.c -o ../../target/examples/miniP_client

target/examples/miniP_server:
	cd examples/miniP && ${CC} ${CFLAGS} miniP_server.c -o ../../target/examples/miniP_server

leader:
	cargo build

API:
	cbindgen --crate network_time_simulator --output c_src/rust_lib.h --lang c
	CARGO_TARGET_DIR=target/lib cargo build --manifest-path src/Cargo.toml
	cp target/lib/debug/libAPI.a c_src/libAPI.a

clean:
	rm -f testing/*
	rm -f c_src/*.o
	rm -f c_src/*.so
	rm -f c_src/*.a
	rm -f examples/miniP_client
	rm -f examples/miniP_server

little_clean:
	cd target/examples && rm -f miniP_server && rm -f miniP_client
	cd testing && rm -f miniP_server && rm -f miniP_client
