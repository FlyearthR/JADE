#include <sys/select.h>
#include <poll.h>
#include <sys/time.h>
#include <stdlib.h>
#include <mqueue.h>
#include <stdio.h>
#include <string.h>
#include <arpa/inet.h>
#include "helper.h"
#include "rust_lib.h"


#define ID id

#define FDI fdi
#define FDO fdo

extern int id;
extern int seed;
extern int fdi;
extern int fdo;

extern FILE* logs;

int send_msg(Message m);

uint64_t receive_msg();

struct timeval get_time();

uint64_t get_u64_time();

struct timeval blocking();

void add_event(struct timeval);

void suppress_event(struct timeval);

int get_random();

void sender(uint64_t pkt_id);

void send_has_to_send(const struct sockaddr *dest_addr, packet_elem *pe);