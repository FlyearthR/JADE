#include <sys/select.h>
#include <sys/time.h>
#include <sys/types.h>
#include <poll.h>
#include <dlfcn.h>
#include <unistd.h>
#include <arpa/inet.h>
#include <errno.h>
#include <sys/socket.h>
#include <time.h>
#include "communication.h"

int random_number = 42;

int empty_fun()
{
        blocking();
        return 1;
}

ssize_t recvfrom(int sockfd, void *buf, size_t len,
                 int flags, struct sockaddr *src_addr,
                 socklen_t *addrlen)
{
        logs("recvfrom called\n");

        // TODO: configure socket as non blocking at opening time
        int flags_s = fcntl(sockfd, F_GETFL, 0);
        if (flags_s == -1)
                return -1;
        fcntl(sockfd, F_SETFL, O_NONBLOCK);
        LIBC_FUNCTION(ssize_t, recvfrom, int sockfd, void *buf, size_t len,
                      int flags, struct sockaddr *src_addr,
                      socklen_t *addrlen);
        // logs("inside recvfrom 3\n");
        int ret;
        do
        {
                logs("inside recvfrom loop\n");
                ret = LIBC_FUNCTION_GET(recvfrom)(sockfd, buf, len, flags, src_addr, addrlen);
                if (ret == -1 && errno != EWOULDBLOCK) {
                        perror("recvfrom: ");
                }
        } while (ret == -1 && errno == EWOULDBLOCK && empty_fun());
        return ret;
}

ssize_t recvmsg(int sockfd, struct msghdr *msg, int flags)
{
        logs("recvmsg called\n");
        int flags_s = fcntl(sockfd, F_GETFL, 0);
        if (flags_s == -1)
                return -1;
        fcntl(sockfd, F_SETFL, O_NONBLOCK);
        LIBC_FUNCTION(ssize_t, recvmsg, int sockfd, struct msghdr *msg, int flags);
        int ret;
        do
        {
                ret = LIBC_FUNCTION_GET(recvmsg)(sockfd, msg, flags);
        }while (ret == -1 && errno == EWOULDBLOCK && empty_fun());
        return ret;
}

int select(int nfds, fd_set *restrict readfds,
           fd_set *restrict writefds, fd_set *restrict exceptfds,
           struct timeval *restrict timeout)
{
        logs("select called\n");
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
        // logs("select called\n");
        cur = blocking();
        while (ret == 0 && before_timeval(cur, to))
        {
                ret = LIBC_FUNCTION_GET(select)(nfds, readfds, writefds, exceptfds, &zeros);
                // logs("select loop called\n");
                cur = blocking();
        }
        if (before_timeval(cur, to))
                suppress_event(to);
        return ret;
}

int pselect(int nfds, fd_set *restrict readfds,
            fd_set *restrict writefds, fd_set *restrict exceptfds,
            const struct timespec *restrict timeout,
            const sigset_t *restrict sigmask)
{
        logs("pselect called\n");
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
        // logs("pselect called\n");
        cur = blocking();
        while (ret == 0 && before_timeval(cur, to))
        {
                ret = LIBC_FUNCTION_GET(pselect)(nfds, readfds, writefds, exceptfds, &zeros, sigmask);
                // logs("pselect loop called\n");
                cur = blocking();
        }
        if (before_timeval(cur, to))
                suppress_event(to);
        return ret;
}

int infinity_poll(struct pollfd *fds, nfds_t nfds, int timeout)
{
        logs("infinity_poll called\n");
        LIBC_FUNCTION(int, poll, struct pollfd *fds, nfds_t nfds, int timeout);
        int ret;
        do
        {
                ret = LIBC_FUNCTION_GET(poll)(fds, nfds, 0);
        } while (ret == 0 && empty_fun());
        return ret;
}

int poll(struct pollfd *fds, nfds_t nfds, int timeout)
{
        logs("poll called\n");
        LIBC_FUNCTION(int, poll, struct pollfd *fds, nfds_t nfds, int timeout);
        if (timeout < 0)
                return infinity_poll(fds, nfds, timeout);
        struct timeval cur = get_time();
        struct timeval to = add_timeval(cur, int_to_timeval_s(timeout));
        int ret = LIBC_FUNCTION_GET(poll)(fds, nfds, 0);
        if (ret)
                return ret;
        add_event(to);
        // logs("poll called\n");
        cur = blocking();
        do
        {
                ret = LIBC_FUNCTION_GET(poll)(fds, nfds, 0);
                // logs("poll loop called\n");
        } while (ret == 0 && before_timeval(cur = blocking(), to));
        if (before_timeval(cur, to))
                suppress_event(to);
        return ret;
}

int ppoll(struct pollfd *fds, nfds_t nfds,
          const struct timespec *tmo_p, const sigset_t *sigmask)
{
        logs("ppoll called\n");
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
        do
        {
                ret = LIBC_FUNCTION_GET(ppoll)(fds, nfds, &zeros, sigmask);
                // logs("ppoll loop called\n");
        } while (ret == 0 && before_timeval(cur = blocking(), to));
        if (before_timeval(cur, to))
                suppress_event(to);
        return ret;
}

int gettimeofday(struct timeval *restrict tv,
                 void *restrict tz)
{
        logs("gettimeofday called\n");
        LIBC_FUNCTION(int, gettimeofday, struct timeval *restrict tv,
                      void *restrict tz);
        int ret = LIBC_FUNCTION_GET(gettimeofday)(tv, tz);
        struct timeval t = get_time();
        tv->tv_sec = t.tv_sec;
        tv->tv_usec = t.tv_usec;
        return ret;
}

unsigned int sleep(unsigned int seconds)
{
        logs("sleep called\n");
        if (seconds == 0)
                return 0;
        struct timeval start = get_time();
        struct timeval end = start;
        end.tv_sec += seconds;
        add_event(end);
        // logs("sleep called (multiple blocking possible)\n");
        while (before_timeval(blocking(), end))
                ;
        // libc: Zero if the requested time has elapsed,
        //   or the number of seconds left to sleep, if the call was  interrupted
        //   by a signal handler.
        //   We do not currently support signals
        //
        return 0;
}

int usleep(useconds_t usec)
{
        logs("usleep called\n");
        if (usec == 0)
                return 0;
        struct timeval start = get_time();
        struct timeval end = {.tv_sec = 0, .tv_usec = usec};
        end = add_timeval(start, end);
        add_event(end);
        // logs("usleep called (multiple blocking possible)\n");
        while (before_timeval(blocking(), end))
                ;
        return 0;
}

void srand(unsigned int local_seed)
{
        logs("srand called\n");
        seed = local_seed;
        random_number = get_random();
}

int rand(void)
{
        logs("gettimeofday called\n");
        return get_random();
}

struct sockaddr *get_ip(int fd){
        fd_ip_elem goal = {.fi= {.fd = fd}};
        fd_ip_elem* found = NULL;
        LL_SEARCH(fd_ip_list,found,&goal,cmp_fd_ip);
        if (!found)
                exit(-12);
        return &found->fi.addr;
}

ssize_t send(int sockfd, const void *buf, size_t len, int flags)
{
        logs("send called\n");
        packet_elem *pe;
        if ((pe = (packet_elem *)malloc(sizeof *pe)) == NULL)
                exit(-13);
        if ((pe->pkt.buf = malloc(len)) == NULL)
                exit(-13);

        pe->pkt.id = next_pkt_id++;
        pe->pkt.tos = send_t;
        pe->pkt.sockfd = sockfd;
        memcpy((void *)pe->pkt.buf, buf, len);
        pe->pkt.len = len;
        pe->pkt.flags = flags;
        pe->pkt.dest_addr = NULL;
        pe->pkt.addrlen = 0;

        LL_PREPEND(pkt_list, pe);

        struct sockaddr* dest_addr = get_ip(sockfd);

        send_has_to_send(dest_addr, pe);

        return len;
}

ssize_t sendto(int sockfd, const void *buf, size_t len, int flags,
               const struct sockaddr *dest_addr, socklen_t addrlen)
{
        logs("sendto called\n");
        packet_elem *pe;
        if ((pe = (packet_elem *)malloc(sizeof *pe)) == NULL)
                exit(-13);
        if ((pe->pkt.buf = malloc(len)) == NULL)
                exit(-13);

        pe->pkt.id = next_pkt_id++;
        pe->pkt.tos = sendto_t;
        pe->pkt.sockfd = sockfd;
        memcpy((void *)pe->pkt.buf, buf, len);
        pe->pkt.len = len;
        pe->pkt.flags = flags;
        if ((pe->pkt.dest_addr = (struct sockaddr *)malloc(addrlen)) == NULL)
                exit(-13);
        memcpy((void *)pe->pkt.dest_addr, dest_addr, addrlen);
        pe->pkt.addrlen = addrlen;

        LL_PREPEND(pkt_list, pe);

        send_has_to_send(dest_addr, pe);
        return len;
}

ssize_t sendmsg(int sockfd, const struct msghdr *msg, int flags)
{
        // TODO
        logs("sendmsg called\n");
        packet_elem *pe;
        if ((pe = (packet_elem *)malloc(sizeof *pe)) == NULL)
                exit(-13);
        if ((pe->pkt.buf = malloc(sizeof(struct msghdr))) == NULL)
                exit(-13);
        logs("malloc ok\n");
        struct msghdr *buf = (struct msghdr *)pe->pkt.buf;
        buf->msg_name = malloc(msg->msg_namelen);
        memcpy(buf->msg_name, msg->msg_name, msg->msg_namelen);
        buf->msg_namelen = msg->msg_namelen;
        buf->msg_iov = (struct iovec *)malloc(sizeof(struct iovec) * msg->msg_iovlen);
        int n_bytes_sent = 0;
        logs("first part ok\n");
        for (int i = 0; i < msg->msg_iovlen; i++)
        {
                size_t len = (msg->msg_iov + i)->iov_len;
                n_bytes_sent += len;
                void *tmp_buf = malloc(len);
                memcpy(tmp_buf, (msg->msg_iov + i)->iov_base, len);
                struct iovec *iov = (struct iovec *)buf->msg_iov;
                iov += i;
                iov->iov_base = tmp_buf;
                iov->iov_len = len;
        }
        logs("copy buffer ok\n");
        buf->msg_iovlen=msg->msg_iovlen;
        buf->msg_control = malloc(msg->msg_controllen);
        memcpy(buf->msg_control, msg->msg_control, msg->msg_controllen);
        buf->msg_controllen = msg->msg_controllen;
        buf->msg_flags = msg->msg_flags;
        pe->pkt.id = next_pkt_id++;
        pe->pkt.tos = sendmsg_t;
        pe->pkt.sockfd = sockfd;
        //pe->pkt.len = sizeof(struct msghdr);
        pe->pkt.flags = flags;
        // pe->pkt.dest_addr = (struct sockaddr*)(buf->msg_name);
        /*if ((pe->pkt.dest_addr = (struct sockaddr *)malloc(sizeof(((struct sockaddr_in*)(msg->msg_name))->sin_addr))) == NULL){
                exit(-13);
        }
        memcpy(pe->pkt.dest_addr,(struct sockaddr *)(msg->msg_name),sizeof(struct sockaddr));
        
        pe->pkt.addrlen = msg->msg_namelen;*/
        logs("copy finished\n");

        LL_PREPEND(pkt_list, pe);

        // struct sockaddr* dest_addr = get_ip(sockfd);

        
        // struct sockaddr* dest_addr = get_ip(sockfd);

        // pe->pkt.dest_addr = &dest_addr;
        // pe->pkt.addrlen=sizeof(dest_addr);

        logs("sockaddr :\n");

        send_has_to_send(buf->msg_name, pe);

        return n_bytes_sent;
}

int bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen){
        logs("bind called\n");
        logs("sockaddr :\n");
        logs("%s\n", inet_ntoa (((struct sockaddr_in*) addr)->sin_addr));

        fd_ip_elem *f = (fd_ip_elem *)malloc(sizeof(fd_ip_elem));
        f->fi.fd = sockfd;
        memcpy(&f->fi.addr, addr, addrlen);
        LL_PREPEND(fd_ip_list, f);

        LIBC_FUNCTION(int, bind, int sockfd, const struct sockaddr *addr, socklen_t addrlen);
        return LIBC_FUNCTION_GET(bind)(sockfd, addr, addrlen);
}

int connect(int sockfd, const struct sockaddr *addr, socklen_t addrlen)
{
        logs("connect called\n");
        logs("sockaddr :\n");
        
        fd_ip_elem *f = (fd_ip_elem *)malloc(sizeof(fd_ip_elem));
        f->fi.fd = sockfd;
        memcpy(&f->fi.addr, addr, addrlen);
        LL_PREPEND(fd_ip_list, f);

        LIBC_FUNCTION(int, connect, int sockfd, const struct sockaddr *addr,
                      socklen_t addrlen);
        return LIBC_FUNCTION_GET(connect)(sockfd, addr, addrlen);
}

int clock_getres(clockid_t clockid, struct timespec *res) // all clocks are based on the leader's one which is microsecond-precise
{
        logs("clock_getres called\n");
        LIBC_FUNCTION(int, clock_getres, clockid_t clockid, struct timespec *res);
        switch (clockid)
        {
                case CLOCK_REALTIME: // TODO: should we support processes that set this clock?
                case CLOCK_REALTIME_ALARM:
                case CLOCK_REALTIME_COARSE:
                case CLOCK_TAI:
                case CLOCK_MONOTONIC:
                case CLOCK_MONOTONIC_COARSE:
                case CLOCK_MONOTONIC_RAW:
                case CLOCK_BOOTTIME:
                case CLOCK_BOOTTIME_ALARM:
                        res->tv_sec = 0;
                        res->tv_nsec = 1000;
                        return 0;
                case CLOCK_PROCESS_CPUTIME_ID:
                        return LIBC_FUNCTION_GET(clock_getres)(clockid, res);
                case CLOCK_THREAD_CPUTIME_ID:
                        return LIBC_FUNCTION_GET(clock_getres)(clockid, res);
        }
        
}

int clock_gettime(clockid_t clockid, struct timespec *tp)
{
        logs("clock_gettime called\n");
        LIBC_FUNCTION(int, clock_gettime, clockid_t clockid, struct timespec *tp);
        switch (clockid)
        {
                case CLOCK_REALTIME: // TODO: should we support processes that set this clock?
                case CLOCK_REALTIME_ALARM:
                case CLOCK_REALTIME_COARSE:
                case CLOCK_TAI:
                case CLOCK_MONOTONIC:
                case CLOCK_MONOTONIC_COARSE:
                case CLOCK_MONOTONIC_RAW:
                case CLOCK_BOOTTIME:
                case CLOCK_BOOTTIME_ALARM:
                        uint64_t t = get_u64_time();
                        tp->tv_sec = t / 1000000;
                        tp->tv_nsec = (t % 1000000) * 1000;
                        return 0;
                case CLOCK_PROCESS_CPUTIME_ID:
                        return LIBC_FUNCTION_GET(clock_gettime)(clockid, tp);
                case CLOCK_THREAD_CPUTIME_ID:
                        return LIBC_FUNCTION_GET(clock_gettime)(clockid, tp);
        }
}

int clock_settime(clockid_t clockid, const struct timespec *tp)
{
        logs("clock_gettime called\n");
        return -1; // TODO: set errno
}

#ifdef DEBUG
int open(const char *pathname, int flags, ...)
{
        va_list ap;
        LIBC_FUNCTION(int, open, const char *pathname, int flags, ...);
        int ret = LIBC_FUNCTION_GET(open)(pathname, flags, ap);
        logs("fd correspondance - fd: %i; path: %s\n", ret, pathname);
        return ret;
}
#endif

ssize_t read(int fd, void *buf, size_t count)
{
        logs("read called - fd: %i\n", fd);
        LIBC_FUNCTION(ssize_t, read, int fd, void *buf, size_t count);
        if (getsockname(fd, NULL, 0) == -1 && errno == ENOTSOCK)
        { // this is not an FD to a socket, we have to pass it to the kernel
                return LIBC_FUNCTION_GET(read)(fd, buf, count);
        }
        // TODO: configure socket as non blocking at opening time
        int flags_s = fcntl(fd, F_GETFL, 0);
        if (flags_s == -1)
                return -1;

        int ret;
        do
        {
                ret = LIBC_FUNCTION_GET(read)(fd, buf, count);
        } while (ret == 0 && empty_fun());
        return ret;
}

ssize_t __read_chk(int fd, void *buf, size_t count)
{
        logs("__read_chk called\n");
        LIBC_FUNCTION(ssize_t, __read_chk, int fd, void *buf, size_t count);
        if (getsockname(fd, NULL, 0) == -1 && errno == ENOTSOCK)
        { // this is not an FD to a socket, we have to pass it to the kernel
                return LIBC_FUNCTION_GET(__read_chk)(fd, buf, count);
        }
        // TODO: configure socket as non blocking at opening time
        int flags_s = fcntl(fd, F_GETFL, 0);
        if (flags_s == -1)
                return -1;

        int ret;
        do
        {
                ret = LIBC_FUNCTION_GET(__read_chk)(fd, buf, count);
        } while (ret == 0 && empty_fun());
        return ret;
}