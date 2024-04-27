CC = gcc
CFLAGS = -g

.PHONY: syscalls.so all test API leader clean little_clean

test: all
	cargo test
	cd examples/ && ./unlink "/nts_mq_0" && ./unlink "/nts_mq_1" && ./unlink "/nts_mq_2"
	cp -f target/debug/leader testing/leader
	cp -f target/syscalls/syscalls.so testing/syscalls.so
	cp -f target/examples/miniP_client test/client
	cp -f target/examples/miniP_server test/server
	cd testing && RUST_BACKTRACE=1 ./leader

all: syscalls.so leader examples API 

syscalls.so: API
	cd c_src && $(MAKE) syscalls.so

examples: target/examples/miniP_client target/examples/miniP_server

target/examples/miniP_client:
	cd examples/miniP && $(CC) ${CFLAGS} miniP_client.c -o ../../target/examples/client

target/examples/miniP_server:
	cd examples/miniP && ${CC} ${CFLAGS} miniP_server.c -o ../../target/examples/server

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
	rm -f examples/client
	rm -f examples/server

little_clean:
	cd target/examples && rm -f miniP_server && rm -f miniP_client
	cd testing && rm -f miniP_server && rm -f miniP_client
