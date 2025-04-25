CC = gcc
CFLAGS = -g -DDEBUG -Wall

%.o: %.c
	$(CC) -fPIC -I. -llibAPI -c $^ -o $@

syscalls.so: communication.o helper.o syscalls.o
	$(CC) -shared -o syscalls.so communication.o helper.o syscalls.o libAPI.a -ldl && mkdir -p ../target/syscalls && cp syscalls.so ../target/syscalls/syscalls.so

test_ffi:
	$(CC) ${CFLAGS} -I. -o tests/test_ffi tests/test_ffi.c libAPI.a -lcunit

static: 
	$(CC) -static -static-libgcc -fPIC -I. -llibAPI -c helper.c -o helper.o
	$(CC) -static -static-libgcc -fPIC -I. -llibAPI -c communication.c -o communication.o
	$(CC) -static -static-libgcc -fPIC -I. -llibAPI -c syscalls.c -o syscalls.o
	$(CC) -shared -o syscalls.so communication.o helper.o syscalls.o libAPI.a -ldl && mkdir -p ../target/syscalls && cp syscalls.so ../target/syscalls/syscalls.so

clean:
	rm *.o && rm *.a


