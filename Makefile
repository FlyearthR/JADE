CC = gcc
CFLAGS = -g
INSTANCE = 9_clients_1_server_random

.PHONY: syscalls.so all test API leader clean little_clean

test: all
	./test.sh

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

testPicoquic: all
	cp -f target/debug/simulator testing/simulator
	cp -f target/syscalls/syscalls.so testing/syscalls.so
	$(CC) ${CFLAGS} examples/simple_client.c -o testing/simple_client
	$(CC) ${CFLAGS} examples/random_client.c -o testing/random_client
	${CC} ${CFLAGS} examples/simple_server.c -o testing/simple_server
	rm -f examples/picoquic/simulator examples/picoquic/syscalls.so examples/picoquic/9_clients_1_server_quic.*
	cp -f tests/9_clients_1_server_quic.* examples/picoquic/
	cp c_src/syscalls.so examples/picoquic/
	cp testing/simulator examples/picoquic/
	cd examples/web_page_quic && python3 create_pages.py
	cd examples/picoquic/ && sudo RUST_BACKTRACE=1 ./simulator 9_clients_1_server_quic.toml

testQuiche: all
	cp -f target/debug/simulator testing/simulator
	cp -f target/syscalls/syscalls.so testing/syscalls.so
	cp examples/quiche/target/debug/examples/http3-client testing/http3-client
	cp examples/quiche/target/debug/examples/http3-server testing/http3-server
	cd testing && mkdir -p examples 
	cd testing/examples && mkdir -p root
	cp examples/web_page_quic/create_pages.py testing/examples/root/create_pages.py
	cd testing/examples/root && python3 create_pages.py
	

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
	rm -f examples/logs/*
	rm -f logs/*

little_clean:
	rm -f testing/*

installTestQuic:
	cd examples && mkdir -p logs && \
	if [ ! -d "picoquic" ]; then \
		git clone https://github.com/private-octopus/picoquic.git; \
	else \
	    cd picoquic && make clean && git pull; \
	fi
	cd examples/picoquic && cmake -DPICOQUIC_FETCH_PTLS=Y .
	cd examples/picoquic && make

installTestQuicClean:
	rm -rf examples/picoquic
	rm -rf examples/logs

installTestQuiche:
	cd examples && mkdir -p logs && \
	if [ ! -d "quiche" ]; then \
		git clone --recursive https://github.com/cloudflare/quiche; \
	else \
	    cd quiche && make clean && git pull; \
	fi
	rm -f examples/quiche/quiche/examples/http3-server.rs
	cp tests/modified-http3-server-quiche.rs examples/quiche/quiche/examples/http3-server.rs
	cd examples/quiche && cargo build --examples

CITest: all
	$(MAKE) test
	$(MAKE) testPicoquic
	$(MAKE) testsyscalls
	$(MAKE) testffi
	$(MAKE) usetest