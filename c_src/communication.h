#include <sys/select.h>
#include <poll.h>
#include <sys/time.h>
#include <stdlib.h>
#include <mqueue.h>
#include <stdio.h>
#include <string.h>
#include <arpa/inet.h>
#include <signal.h>
#include <dlfcn.h>
#include "helper.h"
#include "rust_lib.h"


#define ID id

#define FDI fdi
#define FDO fdo

extern int id;
extern int seed;
extern int fdi;
extern int fdo;


int send_msg(Message m);

uint64_t receive_msg(int update_time);

struct timeval get_time();

uint64_t get_u64_time();

struct timeval blocking();

uint64_t blocking_t();

void add_event(struct timeval);

void add_event_t(uint64_t t);

void suppress_event(struct timeval);

void suppress_event_t(uint64_t t);

int get_random();

void sender(uint64_t pkt_id);

void send_has_to_send(const struct sockaddr *dest_addr, packet_elem *pe);

void __attribute__((destructor)) send_finished();

void print_buffer_hex(const unsigned char *buffer, size_t size);

void print_sendto(int sockfd, const void *buf, size_t len, int flags,
               const struct sockaddr *dest_addr, socklen_t addrlen);