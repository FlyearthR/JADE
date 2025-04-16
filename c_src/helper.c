#include "helper.h"

packet_elem *pkt_list;
fd_ip_elem *fd_ip_list;

uint64_t next_pkt_id;

FILE *log_file;

struct itimerval timer_real;

int before_timeval(struct timeval t1, struct timeval t2)
{
    return t1.tv_sec != t2.tv_sec ? t1.tv_sec < t2.tv_sec : t1.tv_usec < t2.tv_usec;
}

unsigned int timeval_to_uint_us(struct timeval t)
{
    return t.tv_sec * 1000000 + t.tv_usec;
}

struct timeval timespec_to_timeval(struct timespec t1)
{
    struct timeval t = {.tv_sec = t1.tv_sec, .tv_usec = t1.tv_nsec / 1000};
    return t;
}

struct timeval int_to_timeval_s(int t)
{
    struct timeval ret = {.tv_sec = t, .tv_usec = 0};
    return ret;
}

struct timeval u64_to_timeval_us(uint64_t t)
{
    struct timeval ret = {.tv_sec = t / 1000000, .tv_usec = t % 1000000};
    return ret;
}

struct timeval add_timeval(struct timeval t1, struct timeval t2)
{
    struct timeval t = {.tv_sec = t1.tv_sec + t2.tv_sec + (t1.tv_usec + t2.tv_usec) / 1000000,
                        .tv_usec = (t1.tv_usec + t2.tv_usec) % 1000000};
    return t;
}

struct timeval susbstract_timeval(struct timeval t1, struct timeval t2)
{
    struct timeval t = {.tv_sec = t1.tv_sec - t2.tv_sec - (t1.tv_usec<t2.tv_usec ? 1 + (t2.tv_usec - t1.tv_usec)/1000000 : 0),
                        .tv_usec = (t1.tv_usec - t2.tv_usec) % 1000000};
    return t;
}


int cmp_pkt(packet_elem *pe1, packet_elem *pe2)
{
    return pe2->pkt.id - pe1->pkt.id;
}

int cmp_fd_ip(fd_ip_elem *fd_ip1, fd_ip_elem *fd_ip2)
{
    return fd_ip1->fi.fd - fd_ip2->fi.fd;
}