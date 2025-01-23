#ifndef HELPER_H
#define HELPER_H

#include <sys/select.h>
#include <poll.h>
#include <sys/time.h>
#include <time.h>
#include <stdint.h>
#include <dlfcn.h>
#include <sys/socket.h>
#include "utlist.h"

enum type_of_send {
    send_t,
    sendto_t,
    sendmsg_t
};

typedef struct packet {
    uint64_t id;
    enum type_of_send tos;
    int sockfd;
    const void* buf;
    size_t len;
    int flags;
    const struct sockaddr *dest_addr;
    socklen_t addrlen;
} packet;

typedef struct packet_elem {
    packet pkt;
    struct packet_elem *next;
} packet_elem;

typedef struct fd_ip {
    int fd;
    const struct sockaddr addr;
} fd_ip;

typedef struct fd_ip_elem {
    fd_ip fi;
    struct fd_ip_elem *next;
}  fd_ip_elem;

extern packet_elem* pkt_list;

extern fd_ip_elem* fd_ip_list;

extern uint64_t next_pkt_id;

int before_timeval(struct timeval, struct timeval);

unsigned int timeval_to_uint_us(struct timeval);

struct timeval timespec_to_timeval(struct timespec);

struct timeval int_to_timeval_s(int);

struct timeval u64_to_timeval_us(uint64_t);

struct timeval add_timeval(struct timeval, struct timeval);

int cmp_pkt(packet_elem* pe1, packet_elem* pe2);

int cmp_fd_ip(fd_ip_elem* fd_ip1,fd_ip_elem* fd_ip2);

#define LIBC_FUNCTION(ftype, fname, ...) ftype (* libc_##fname ) ( __VA_ARGS__ ); \
    libc_##fname = dlsym(RTLD_NEXT, #fname )
#define LIBC_FUNCTION_GET(fname) libc_##fname

#endif