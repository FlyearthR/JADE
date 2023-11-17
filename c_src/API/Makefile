CC = gcc
CFLAGS = -g

%.o: %.c
	$(CC) -fPIC -I. -llibAPI -c $^ -o $@

syscalls.so: communication.o helper.o syscalls.o
	$(CC) -fPIC -shared -o syscalls.so communication.o helper.o syscalls.o libAPI.a -ldl


