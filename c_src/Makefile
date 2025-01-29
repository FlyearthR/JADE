CC = gcc
CFLAGS = -g -DDEBUG

%.o: %.c
	$(CC) -fPIC -I. -llibAPI -c $^ -o $@

syscalls.so: communication.o helper.o syscalls.o
	$(CC) -shared -o syscalls.so communication.o helper.o syscalls.o libAPI.a -ldl && mkdir -p ../target/syscalls && cp syscalls.so ../target/syscalls/syscalls.so

test_ffi:
	$(CC) ${CFLAGS} -I. -lcunit -o test_ffi test_ffi.c libAPI.a

clean:
	rm *.o && rm *.a


