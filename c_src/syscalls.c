#include <sys/select.h>
#include <poll.h>
#include <sys/time.h>
#include <sys/types.h>
#include <dlfcn.h>
#include <unistd.h>
#include <arpa/inet.h>
#include "helper.h"
#include "communication.h"

int random_number = 42;

int inline empty_fun()
{
        blocking();
        return 1;
}

ssize_t recvfrom(int sockfd, void* buf, size_t len,
                        int flags, struct sockaddr * src_addr,
                        socklen_t * addrlen)
{
        // TODO: configure socket as non blocking at opening time
        int flags_s = fcntl(sockfd, F_GETFL, 0);
        if (flags_s == -1)
                return -1;
        fcntl(sockfd, F_SETFL, flags_s|O_NONBLOCK);
        LIBC_FUNCTION(ssize_t, recvfrom, int sockfd, void* buf, size_t len,
                        int flags, struct sockaddr * src_addr,
                        socklen_t * addrlen);
        int ret;
        do {
                ret = LIBC_FUNCTION_GET(recvfrom)(sockfd, buf, len, flags, src_addr, addrlen);
        } while (ret == -1 && empty_fun());
        return ret;
}

int select(int nfds, fd_set *restrict readfds,
                  fd_set *restrict writefds, fd_set *restrict exceptfds,
                  struct timeval *restrict timeout)
{
	struct timeval zeros = {.tv_sec = 0, .tv_usec = 0};
	struct timeval cur = get_time();
	struct timeval to = add_timeval(cur, *timeout);
	LIBC_FUNCTION(int, select, int nfds, fd_set *restrict readfds,
                  fd_set *restrict writefds, fd_set *restrict exceptfds,
                  struct timeval *restrict timeout);
	int ret = LIBC_FUNCTION_GET(select)(nfds, readfds, writefds, exceptfds, &zeros);
	if (ret)
		return ret;
        
	add_event(to);
	cur = blocking();
	while (ret == 0 && before_timeval(cur, to)) {
		ret = LIBC_FUNCTION_GET(select)(nfds, readfds, writefds, exceptfds, &zeros);
		cur = blocking();
	} 
	suppress_event(to);
	return ret;
}

int pselect(int nfds, fd_set *restrict readfds,
                  fd_set *restrict writefds, fd_set *restrict exceptfds,
                  const struct timespec *restrict timeout,
                  const sigset_t *restrict sigmask)
{
        struct timespec zeros = {.tv_sec = 0, .tv_nsec = 0};
        struct timeval cur = get_time();
        struct timeval to = add_timeval(cur, timespec_to_timeval(*timeout));
	LIBC_FUNCTION(int, pselect, int nfds, fd_set *restrict readfds,
                  fd_set *restrict writefds, fd_set *restrict exceptfds,
                  const struct timespec *restrict timeout,
                  const sigset_t *restrict sigmask);
        int ret = LIBC_FUNCTION_GET(pselect)(nfds, readfds, writefds, exceptfds, &zeros, sigmask);
        if (ret)
                return ret;
	add_event(to);
        cur = blocking();
	while (ret == 0 && before_timeval(cur, to)) {
                ret = LIBC_FUNCTION_GET(pselect)(nfds, readfds, writefds, exceptfds, &zeros, sigmask);
		cur = blocking();
	}
	suppress_event(to);
        return ret;
}

int infinity_poll(struct pollfd *fds, nfds_t nfds, int timeout)
{
	LIBC_FUNCTION(int, poll, struct pollfd *fds, nfds_t nfds, int timeout);
	int ret;
        do {
                ret = LIBC_FUNCTION_GET(poll)(fds, nfds, 0);
        } while (ret == 0 && empty_fun());
        return ret;
}

int poll(struct pollfd *fds, nfds_t nfds, int timeout)
{
	LIBC_FUNCTION(int, poll, struct pollfd *fds, nfds_t nfds, int timeout);
	if (timeout < 0)
                return infinity_poll(fds, nfds, timeout);
        struct timeval cur = get_time();
        struct timeval to = add_timeval(cur, int_to_timeval_s(timeout));
	int ret = LIBC_FUNCTION_GET(poll)(fds, nfds, 0);
        if (ret)
                return ret;
        add_event(to);
        cur = blocking();
	do {
                ret = LIBC_FUNCTION_GET(poll)(fds, nfds, 0);
	} while (ret == 0 && before_timeval(blocking(), to));
	suppress_event(to);
        return ret;
}

int ppoll(struct pollfd *fds, nfds_t nfds,
                 const struct timespec *tmo_p, const sigset_t *sigmask)
{
        struct timespec zeros = {.tv_sec = 0, .tv_nsec = 0};
        struct timeval cur = get_time();
        struct timeval to = add_timeval(cur, timespec_to_timeval(*tmo_p));
	LIBC_FUNCTION(int, ppoll, struct pollfd *fds, nfds_t nfds,
                 const struct timespec *tmo_p, const sigset_t *sigmask);
        int ret = LIBC_FUNCTION_GET(ppoll)(fds, nfds, &zeros, sigmask);
        if (ret)
                return ret;
        add_event(to);
        cur = blocking();
	do {
                ret = LIBC_FUNCTION_GET(ppoll)(fds, nfds, &zeros, sigmask);
	} while (ret == 0 && before_timeval(blocking(), to));
        suppress_event(to);
        return ret;
}

int gettimeofday(struct timeval *restrict tv,
                        void * restrict tz)
{       
	LIBC_FUNCTION(int, gettimeofday, struct timeval *restrict tv,
                        void * restrict tz);
	int ret = LIBC_FUNCTION_GET(gettimeofday)(tv, tz);
	struct timeval t = get_time();
	tv->tv_sec = t.tv_sec;
	tv->tv_usec = t.tv_usec;
	return ret;
}

unsigned int sleep(unsigned int seconds)
{
        fflush(stdout);
        fflush(stderr);
        struct timeval start = get_time();
        fflush(stdout);
        fflush(stderr);
        struct timeval end = start;
        end.tv_sec += seconds;
        add_event(end);
        while(before_timeval(blocking(), end));
        /* libc: Zero if the requested time has elapsed,
           or the number of seconds left to sleep, if the call was  interrupted
           by a signal handler.
           We do not currently support signals
        */
        return 0;
}

int usleep(useconds_t usec)
{
        struct timeval start = get_time();
        struct timeval end = {.tv_sec = 0, .tv_usec = usec};
        end = add_timeval(start, end);
        add_event(end);
        while(before_timeval(blocking(), end));
        return 0;
}

void srand(unsigned int seed)
{
        random_number = get_random();
}

int rand(void)
{
        return random_number;
}

ssize_t send(int sockfd, const void buf, size_t len, int flags)
{
        packet_elem* pe;
        if ((pe = (packet_elem*)malloc(sizeof *pe)) == NULL) exit(-13);
        if ((pe->pkt.buf = malloc(len)) == NULL) exit(-13);

        pe->pkt.id = next_pkt_id++;
        pe->pkt.sockfd = sockfd;
        memcpy(pe->pkt.buf, buf, len);
        pe->pkt.len = len;
        pe->pkt.flags = flags;

        LL_PREPEND(pkt_list, pe);
        return len;
}