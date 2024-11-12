CC = gcc
CFLAGS = -g

%.o: %.c
	$(CC) -fPIC -I. -llibAPI -c $^ -o $@

syscalls.so: communication.o helper.o syscalls.o
	$(CC) -shared -o syscalls.so communication.o helper.o syscalls.o libAPI.a -ldl && mv syscalls.so ../target/syscalls/syscalls.so && rm *.o && rm *.a


