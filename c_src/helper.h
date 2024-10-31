#include <sys/select.h>
#include <poll.h>
#include <sys/time.h>
#include <time.h>
#include <stdint.h>
#include "utlist.h"

typedef struct packet {
    uint64_t id;
    int sockfd;
    const void* buf;
    size_t len;
    int flags;
} packet;

typedef struct packet_elem {
    packet pkt;
    struct packet_elem *next;
} packet_elem;

packet_elem* pkt_list;

uint64_t next_pkt_id;

int before_timeval(struct timeval, struct timeval);

unsigned int timeval_to_uint_us(struct timeval);

struct timeval timespec_to_timeval(struct timespec);

struct timeval int_to_timeval_s(int);

struct timeval u64_to_timeval_us(uint64_t);

struct timeval add_timeval(struct timeval, struct timeval);

int cmp_pkt(packet_elem pe1, packet_elem pe2);

#define LIBC_FUNCTION(ftype, fname, ...) ftype (* libc_##fname ) ( __VA_ARGS__ ); \
    libc_##fname = dlsym(RTLD_NEXT, #fname )
#define LIBC_FUNCTION_GET(fname) libc_##fname