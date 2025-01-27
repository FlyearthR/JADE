CC = gcc
CFLAGS = -g
INSTANCE = 9_clients_1_server_random

.PHONY: syscalls.so all test API leader clean little_clean

test: all
	cargo test --package network_time_simulator --bin simulator -- unit_testing --show-output --nocapture
	cargo test --package network_time_simulator --bin simulator -- determinism --show-output --nocapture

diff: all
	$(MAKE) usetest | grep Sen[dt]\( > testing/sim.1.sendt.log
	cp testing/test.log testing/sim.1.test.log
	$(MAKE) usetest | grep Sen[dt]\( > testing/sim.2.sendt.log
	cp testing/test.log testing/sim.2.test.log
	./tests/compare_diff.sh

usetest: all
	cp -f target/debug/simulator testing/simulator
	cp -f target/syscalls/syscalls.so testing/syscalls.so
	$(CC) ${CFLAGS} examples/simple_client.c -o testing/simple_client
	$(CC) ${CFLAGS} examples/random_client.c -o testing/random_client
	${CC} ${CFLAGS} examples/simple_server.c -o testing/simple_server
	cp -f tests/$(INSTANCE).* testing/
	cd testing && sudo RUST_BACKTRACE=1 ./simulator $(INSTANCE).toml

testQuic: all
	cp -f target/debug/simulator testing/simulator
	cp -f target/syscalls/syscalls.so testing/syscalls.so
	$(CC) ${CFLAGS} examples/simple_client.c -o testing/simple_client
	$(CC) ${CFLAGS} examples/random_client.c -o testing/random_client
	${CC} ${CFLAGS} examples/simple_server.c -o testing/simple_server
	rm -f examples/picoquic/simulator examples/picoquic/syscalls.so examples/picoquic/9_clients_1_server_quic.*
	cp -f tests/9_clients_1_server_quic.* examples/picoquic/
	cp c_src/syscalls.so examples/picoquic/
	cp testing/simulator examples/picoquic/
	cd examples/picoquic/ && sudo RUST_BACKTRACE=1 ./simulator 9_clients_1_server_quic.toml

testffi: API
	cd c_src && $(MAKE) test_ffi && cp test_ffi ../testing/test_ffi
	./testing/test_ffi

testsyscalls: all
	cd c_src/tests && ./tester.sh sendto

all: syscalls.so simulator API 

syscalls.so: API
	cd c_src && $(MAKE) syscalls.so

examples: target/examples/miniP_client target/examples/miniP_server target/examples/simple_client target/examples/simple_server

target/examples/miniP_client:
	cd examples/miniP && $(CC) ${CFLAGS} miniP_client.c -o ../../target/examples/miniP_client

target/examples/miniP_server:
	cd examples/miniP && ${CC} ${CFLAGS} miniP_server.c -o ../../target/examples/miniP_server

target/examples/random_client:
	cd examples/ && $(CC) ${CFLAGS} random_client.c -o ../../target/examples/random_client

target/examples/simple_client:
	cd examples/ && $(CC) ${CFLAGS} simple_client.c -o ../../target/examples/simple_client

target/examples/simple_server:
	cd examples/ && ${CC} ${CFLAGS} simple_server.c -o ../../target/examples/simple_server

simulator:
	cargo build

API:
	#cargo install cbindgen
	cbindgen --crate network_time_simulator --output c_src/rust_lib.h --lang c
	CARGO_TARGET_DIR=target/lib cargo build --manifest-path src/Cargo.toml
	cp target/lib/debug/libapi.a c_src/libAPI.a

clean:
	rm -f testing/*
	rm -f c_src/*.o
	rm -f c_src/*.so
	rm -f c_src/*.a
	rm -f examples/*_client
	rm -f examples/*_server
	rm -rf target/syscalls

little_clean:
	rm -f testing/*
