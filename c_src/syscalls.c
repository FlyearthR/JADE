#include <sys/epoll.h>
#include <sys/select.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <sys/types.h>
#include <poll.h>
#include <unistd.h>
#include <arpa/inet.h>
#include <errno.h>
#include <time.h>
#include <dlfcn.h>
#include "communication.h"

int random_number = 42;

int empty_fun()
{
    blocking();
    return 1;
}

int cmp_fd_ip_ip(fd_ip_elem *a, fd_ip_elem *b)
{
    return a->fi.fd - b->fi.fd;
}

struct sockaddr *get_ip(int fd)
{
    fd_ip_elem goal = {.fi = {.fd = fd}};
    fd_ip_elem *found = NULL;
    LOGS("get_ip fd: %i\n", fd);
    LL_SEARCH(fd_ip_list, found, &goal, cmp_fd_ip_ip);
    if (!found) {
        perror("ip not found");
        printf("bad ip\n");
        fflush(stdout);
        LOGS("no ip found\n");
        exit(-12);
    }
    return &found->fi.addr;
}

ssize_t recvfrom(int sockfd, void *buf, size_t len,
    int flags, struct sockaddr *src_addr,
    socklen_t *addrlen)
{
    LOGS("recvfrom called\n");

    // TODO: configure socket as non blocking at opening time
    int flags_s = fcntl(sockfd, F_GETFL, 0);
    if (flags_s == -1)
    return -1;
    fcntl(sockfd, F_SETFL, O_NONBLOCK);
    LIBC_FUNCTION(ssize_t, recvfrom, int sockfd, void *buf, size_t len,
        int flags, struct sockaddr *src_addr,
        socklen_t *addrlen);
    int ret;
    do
    {
        LOGS("inside recvfrom loop\n");
        ret = LIBC_FUNCTION_GET(recvfrom)(sockfd, buf, len, flags, src_addr, addrlen);
        if (ret == -1 && errno != EWOULDBLOCK)
        {
            perror("recvfrom: ");
        }
    } while (ret == -1 && errno == EWOULDBLOCK && empty_fun());
    LOGS("will quit recvfrom\n");
    return ret;
}


ssize_t recvmsg(int sockfd, struct msghdr *msg, int flags)
{
    LOGS("recvmsg called\n");
    int flags_s = fcntl(sockfd, F_GETFL, 0);
    if (flags_s == -1)
        return -1;
    fcntl(sockfd, F_SETFL, O_NONBLOCK);
    LIBC_FUNCTION(ssize_t, recvmsg, int sockfd, struct msghdr *msg, int flags);
    int ret;
    do
    {
        ret = LIBC_FUNCTION_GET(recvmsg)(sockfd, msg, flags);
    } while (ret == -1 && errno == EWOULDBLOCK && empty_fun());
    return ret;
}


int select(int nfds, fd_set *restrict readfds,
    fd_set *restrict writefds, fd_set *restrict exceptfds,
    struct timeval *restrict timeout) // TODO: support null pointer timeout as infinite select
{
    LOGS("select called\n");
    struct timeval zeros = {.tv_sec = 0, .tv_usec = 0};
    uint64_t cur = get_u64_time();
    uint64_t to = cur + timeval_to_uint_us(*timeout);
    LIBC_FUNCTION(int, select, int nfds, fd_set *restrict readfds,
            fd_set *restrict writefds, fd_set *restrict exceptfds,
            struct timeval *restrict timeout);
    int ret = LIBC_FUNCTION_GET(select)(nfds, readfds, writefds, exceptfds, &zeros);
    if (ret)
        return ret;

    add_event_t(to);
    cur = blocking_t();
    while (ret == 0 && cur < to)
    {
        ret = LIBC_FUNCTION_GET(select)(nfds, readfds, writefds, exceptfds, &zeros);
        cur = blocking_t();
    }
    if (cur < to)
        suppress_event_t(to);
    return ret;
}

int pselect(int nfds, fd_set *restrict readfds,
    fd_set *restrict writefds, fd_set *restrict exceptfds,
    const struct timespec *restrict timeout,
    const sigset_t *restrict sigmask)
{
    LOGS("pselect called\n");
    struct timespec zeros = {.tv_sec = 0, .tv_nsec = 0};
    uint64_t cur = get_u64_time();
    uint64_t to = cur + timeval_to_uint_us(timespec_to_timeval(*timeout));
    LIBC_FUNCTION(int, pselect, int nfds, fd_set *restrict readfds,
            fd_set *restrict writefds, fd_set *restrict exceptfds,
            const struct timespec *restrict timeout,
            const sigset_t *restrict sigmask);
    int ret = LIBC_FUNCTION_GET(pselect)(nfds, readfds, writefds, exceptfds, &zeros, sigmask);
    if (ret)
        return ret;
    add_event_t(to);
    cur = blocking_t();
    while (ret == 0 && cur < to)
    {
        ret = LIBC_FUNCTION_GET(pselect)(nfds, readfds, writefds, exceptfds, &zeros, sigmask);
        cur = blocking_t();
    }
    if (cur < to)
        suppress_event_t(to);
    return ret;
}

int infinity_poll(struct pollfd *fds, nfds_t nfds, int timeout)
{
    LOGS("infinity_poll called\n");
    LIBC_FUNCTION(int, poll, struct pollfd *fds, nfds_t nfds, int timeout);
    int ret;
    do
    {
        ret = LIBC_FUNCTION_GET(poll)(fds, nfds, 0);
    } while (ret == 0 && empty_fun());
    return ret;
}

int infinity_epoll(int epfd, struct epoll_event *events, int maxevents, int timeout)
{
    LOGS("infinity_epoll called\n");
    LIBC_FUNCTION(int, epoll_wait, int epfd, struct epoll_event *events, int maxevents, int timeout);
    int ret;
    int n_events = 0;
    do
    {
        ret = LIBC_FUNCTION_GET(epoll_wait)(epfd, events, maxevents-n_events, 0);
        n_events += ret;
    } while (ret == 0 && empty_fun() && n_events < maxevents);
    return ret;
}

int poll(struct pollfd *fds, nfds_t nfds, int timeout)
{
    LOGS("poll called\n");
    LIBC_FUNCTION(int, poll, struct pollfd *fds, nfds_t nfds, int timeout);
    if (timeout < 0)
        return infinity_poll(fds, nfds, timeout);
    uint64_t cur = get_u64_time();
    uint64_t to = cur + timeout*1000;
    int ret = LIBC_FUNCTION_GET(poll)(fds, nfds, 0);
    if (ret)
        return ret;
    add_event_t(to);
    cur = blocking_t();
    do
    {
        ret = LIBC_FUNCTION_GET(poll)(fds, nfds, 0);
    } while (ret == 0 && (cur = blocking_t()) < to);
    if (cur < to)
        suppress_event_t(to);
    return ret;
}

int ppoll(struct pollfd *fds, nfds_t nfds,
          const struct timespec *tmo_p, const sigset_t *sigmask)
{
    LOGS("ppoll called\n");
    struct timespec zeros = {.tv_sec = 0, .tv_nsec = 0};
    uint64_t cur = get_u64_time();
    uint64_t to = cur + timeval_to_uint_us(timespec_to_timeval(*tmo_p));
    LIBC_FUNCTION(int, ppoll, struct pollfd *fds, nfds_t nfds,
                  const struct timespec *tmo_p, const sigset_t *sigmask);
    int ret = LIBC_FUNCTION_GET(ppoll)(fds, nfds, &zeros, sigmask);
    if (ret)
        return ret;
    add_event_t(to);
    cur = blocking_t();
    do
    {
        ret = LIBC_FUNCTION_GET(ppoll)(fds, nfds, &zeros, sigmask);
    } while (ret == 0 && (cur = blocking_t()) < to);
    if (cur < to)
        suppress_event_t(to);
    return ret;
}

int epoll_wait(int epfd, struct epoll_event *events, int maxevents, int timeout){
    LOGS("epoll_wait called\n");
    LIBC_FUNCTION(int, epoll_wait, int epfd, struct epoll_event *events, int maxevents, int timeout);
    if (timeout < 0)
        return infinity_epoll(epfd, events, maxevents, timeout);
        uint64_t cur = get_u64_time();
        uint64_t to = cur + timeout*1000;
    int ret = LIBC_FUNCTION_GET(epoll_wait)(epfd, events, maxevents, 0);
    if (ret)
        return ret;
    add_event_t(to);
    cur = blocking_t();
    int n_events = 0;
    do
    {
        ret = LIBC_FUNCTION_GET(epoll_wait)(epfd, events, maxevents-n_events, 0);
        n_events += ret;
    } while (ret == 0 && (cur = blocking_t()) < to && n_events < maxevents);
    return 0;
}

time_t time (time_t *__timer)
{
    if (__timer != NULL)
        *__timer = get_u64_time()/1000000;
    return get_u64_time()/1000000;
}

int gettimeofday(struct timeval *restrict tv,
                 void *restrict tz)
{
    LOGS("gettimeofday called\n");
    LIBC_FUNCTION(int, gettimeofday, struct timeval *restrict tv,
                  void *restrict tz);
    int ret = LIBC_FUNCTION_GET(gettimeofday)(tv, tz);
    struct timeval t = get_time();
    tv->tv_sec = t.tv_sec;
    tv->tv_usec = t.tv_usec;
    return ret;
}

int clock_getres(clockid_t clockid, struct timespec *res) // all clocks are based on the leader's one which is microsecond-precise
{
    LOGS("clock_getres called\n");
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
    LOGS("clock_gettime called\n");
    LIBC_FUNCTION(int, clock_gettime, clockid_t clockid, struct timespec *tp);
    uint64_t t;
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
        t = get_u64_time();
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
    LOGS("clock_gettime called\n");
    return -1; // TODO: set errno
}

struct tm *gmtime_r(const time_t *timep, struct tm *result){
    LOGS("gmtime_r called\n");
    LIBC_FUNCTION(struct tm *, gmtime_r, const time_t *timep, struct tm *result);
    struct tm *ret = LIBC_FUNCTION_GET(gmtime_r)(timep, result);
    struct timeval t = get_time();
    result->tm_sec = t.tv_sec;
    result->tm_min = t.tv_sec / 60;
    result->tm_hour = t.tv_sec / 3600;
    result->tm_mday = t.tv_sec / 86400;
    result->tm_mon = t.tv_sec / 2592000;
    result->tm_year = t.tv_sec / 31536000;
    result->tm_wday = t.tv_sec / 86400 % 7;
    result->tm_yday = t.tv_sec / 86400 % 365;
    result->tm_isdst = -1;
    return ret;
}

int setitimer(int which, const struct itimerval *restrict new_value,
    struct itimerval * restrict old_value)
{
    switch (which) {
        case ITIMER_REAL:
        if (old_value != NULL) {
            old_value->it_value = susbstract_timeval(timer_real.it_value, get_time());
            old_value->it_interval = timer_real.it_interval;
        }
        timer_real.it_value = add_timeval(get_time(), new_value->it_value);
        timer_real.it_interval = new_value->it_interval;
        return 0;
        add_event(timer_real.it_value);
        case ITIMER_VIRTUAL:
            LOGS("setitimer called with ITIMER_VIRTUAL, but computational time is considered as zero by simulation\n");
            errno = EINVAL;
            return -1;
        case ITIMER_PROF:
            LOGS("setitimer called with ITIMER_PROF, but computational time is considered as zero by simulation\n");
            errno = EINVAL;
            return -1;
        default:
            LOGS("setitimer called with unknown timer\n");
            errno = EINVAL;
            return -1;
    }
}

int bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen) // TODO: should we intercept this call?
{
    LOGS("bind called\n");
    LOGS("sockaddr: %s, fd: %i\n", inet_ntoa(((struct sockaddr_in *)addr)->sin_addr), sockfd);

    // bind associate an fd with a SOURCE ip, our data structure match fd to DESTINATION ip
    /*fd_ip_elem *f = (fd_ip_elem *)malloc(sizeof(fd_ip_elem));
    f->fi.fd = sockfd;
    memcpy(&f->fi.addr, addr, addrlen);
    LL_PREPEND(fd_ip_list, f);*/

    LIBC_FUNCTION(int, bind, int sockfd, const struct sockaddr *addr, socklen_t addrlen);
    return LIBC_FUNCTION_GET(bind)(sockfd, addr, addrlen);
}

int connect(int sockfd, const struct sockaddr *addr, socklen_t addrlen) // TODO: check if still used
{
    LOGS("connect called\n");
    LOGS("sockaddr :\n");
    LOGS("%s\n", inet_ntoa(((struct sockaddr_in *)addr)->sin_addr));

    fd_ip_elem *f = (fd_ip_elem *)malloc(sizeof(fd_ip_elem));
    f->fi.fd = sockfd;
    memcpy(&f->fi.addr, addr, addrlen);
    fd_ip_elem* elem;
    LOGS("list before prepend\n");
    LL_FOREACH(fd_ip_list, elem
    {
        LOGS("fd: %i, ip: %s\n", elem->fi.fd, inet_ntoa(((struct sockaddr_in*)&(elem->fi.addr))->sin_addr));
    }
    LL_PREPEND(fd_ip_list, f);
    LOGS("list after prepend\n");
    LL_FOREACH(fd_ip_list, elem)
    {
        LOGS("fd: %i, ip: %s\n", elem->fi.fd, inet_ntoa(((struct sockaddr_in*)&(elem->fi.addr))->sin_addr));
    }

    LOGS("prepending fd: %i, ip: %s\n", sockfd, inet_ntoa(((struct sockaddr_in *)addr)->sin_addr));

    LIBC_FUNCTION(int, connect, int sockfd, const struct sockaddr *addr,
                  socklen_t addrlen);
    return LIBC_FUNCTION_GET(connect)(sockfd, addr, addrlen);
}

int socket(int domain, int type, int protocol){
    LOGS("socket called\n");
    LIBC_FUNCTION(int, socket, int domain, int type, int protocol);
    int ret = LIBC_FUNCTION_GET(socket)(domain, type|O_NONBLOCK, protocol);
    return ret;
}


ssize_t send(int sockfd, const void *buf, size_t len, int flags)
{
    LOGS("send called\n");
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

    LOGS("send called 2\n");
    LL_PREPEND(pkt_list, pe);
    LOGS("send called 3\n");

    struct sockaddr *dest_addr = get_ip(sockfd);

    LOGS("send called 4\n");
    send_has_to_send(dest_addr, pe);

    return len;
}

ssize_t sendmsg(int sockfd, const struct msghdr *msg, int flags)
{
    LOGS("sendmsg called\n");
    packet_elem *pe;
    if ((pe = (packet_elem *)malloc(sizeof *pe)) == NULL)
        exit(-13);
    if ((pe->pkt.buf = malloc(sizeof(struct msghdr))) == NULL)
        exit(-13);

    struct msghdr *buf = (struct msghdr *)pe->pkt.buf;
    buf->msg_name = malloc(msg->msg_namelen);
    memcpy(buf->msg_name, msg->msg_name, msg->msg_namelen);
    buf->msg_namelen = msg->msg_namelen;
    buf->msg_iov = (struct iovec *)malloc(sizeof(struct iovec) * msg->msg_iovlen);
    int n_bytes_sent = 0;

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

    buf->msg_iovlen = msg->msg_iovlen;
    buf->msg_control = malloc(msg->msg_controllen);
    memcpy(buf->msg_control, msg->msg_control, msg->msg_controllen);
    buf->msg_controllen = msg->msg_controllen;
    buf->msg_flags = msg->msg_flags;
    pe->pkt.id = next_pkt_id++;
    pe->pkt.tos = sendmsg_t;
    pe->pkt.sockfd = sockfd;
    pe->pkt.flags = flags;

    LL_PREPEND(pkt_list, pe);

    send_has_to_send(buf->msg_name, pe);

    return n_bytes_sent;
}

ssize_t sendto(int sockfd, const void *buf, size_t len, int flags,
               const struct sockaddr *dest_addr, socklen_t addrlen)
{
    LOGS("sendto called\n");
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

void abort()
{
    send_finished();
    // TODO: let config tell if we abord the whole simulation
    // TODO: decide if we add another parameter to finished message to let know that we aborted
    exit(-1);
}

int pause(void){
    // TODO: support signals
    return -1;
}

int system(const char *cmd)
{
    char buffer[20];
    snprintf(buffer, 20, "%i", fileno(log_file));
    setenv("LOG_FILE_FD", buffer, 1);
    snprintf(buffer, 20, "%lu", get_u64_time());
    setenv("CURRENT_TIME", buffer, 1);
    LIBC_FUNCTION(int, system, const char *cmd);
    int ret = LIBC_FUNCTION_GET(system)(cmd);
    unsetenv("CURRENT_TIME");
    unsetenv("LOG_FILE_FD");
    return ret;
}

#ifdef DEBUG
int open(const char *pathname, int flags, ...)
{
    LOGS("open called\n");
    va_list ap;
    LIBC_FUNCTION(int, open, const char *pathname, int flags, ...);
    int ret = LIBC_FUNCTION_GET(open)(pathname, flags, ap);
    LOGS("fd correspondance - fd: %i; path: %s\n", ret, pathname);
    return ret;
}
#endif

#define GET_FD(fd, ...) fd

#define READ_IMPLEM(ret, func, ...) \
    if (getsockname(GET_FD(__VA_ARGS__), NULL, 0) == -1 && errno == ENOTSOCK) \
    { /* this is not an FD to a socket, we have to pass it to the kernel */ \
        ret = func(__VA_ARGS__); \
    } \
    else \
    { \
        /* TODO: configure socket as non blocking at opening time */ \
        int flags_s = fcntl(GET_FD(__VA_ARGS__), F_GETFL, 0); \
        if (flags_s == -1) \
        { \
            ret = -1; \
        } \
        else \
        { \
            do \
            { \
                ret = func(__VA_ARGS__); \
            } while (ret == 0 && empty_fun()); \
        } \
    }

#define read_implem(f, ...) \
({ ssize_t _ret; \
READ_IMPLEM(_ret, f, __VA_ARGS__); \
_ret;})

ssize_t __read_chk(int fd, void *buf, size_t count)
{
    LOGS("__read_chk called\n");
    LIBC_FUNCTION(ssize_t, __read_chk, int fd, void *buf, size_t count);
    return read_implem(LIBC_FUNCTION_GET(__read_chk), fd, buf, count);
}

ssize_t read(int fd, void *buf, size_t count)
{
    LOGS("read called - fd: %i\n", fd);
    LIBC_FUNCTION(ssize_t, read, int fd, void *buf, size_t count);
    return read_implem(LIBC_FUNCTION_GET(read), fd, buf, count);
}

ssize_t pread(int fd, void* buf, size_t count,
    off_t offset)
{
    LOGS("pread called - fd: %i\n", fd);
    LIBC_FUNCTION(ssize_t, pread, int fd, void *buf, size_t count, off_t offset);
    return read_implem(LIBC_FUNCTION_GET(pread), fd, buf, count, offset);
}

int close(int fd){ // TODO: check if still used
    LOGS("close called\n");
    LIBC_FUNCTION(int, close, int fd);
    fd_ip_elem goal = {.fi = {.fd = fd}};
    fd_ip_elem *found = NULL;
    LL_SEARCH(fd_ip_list, found, &goal, cmp_fd_ip_ip);
    if (found)
    {
        LL_DELETE(fd_ip_list, found);
        free(found);
    }
    return LIBC_FUNCTION_GET(close)(fd);
}

ssize_t write(int fd, const void *buf, size_t count)
{
    if (getsockname(fd, NULL, 0) == -1 && errno == ENOTSOCK) 
    { // this is not an FD to a socket, we have to pass it to the kernel
        LIBC_FUNCTION(ssize_t, write, int fd, const void *buf, size_t count);
        return LIBC_FUNCTION_GET(write)(fd, buf, count); 
    }
    LOGS("write called, will call send");
    send(fd, buf, count, 0);
    return 0;
}

ssize_t writev(int fd, const struct iovec *iov, int iovcnt){
    LOGS("writev called\n");
    if (getsockname(fd, NULL, 0) == -1 && errno == ENOTSOCK){
        return writev(fd, iov, iovcnt);
    }
    packet_elem *pe;
    if ((pe = (packet_elem *)malloc(sizeof *pe)) == NULL)
        exit(-13);
    struct iovec *buf;
    if ((buf = (struct iovec *)malloc(sizeof(struct iovec) * iovcnt)) == NULL)
        exit(-13);
    int n_bytes_sent = 0;
    for (int i = 0; i < iovcnt; i++)
    {
        buf[i].iov_base = malloc((iov + i)->iov_len);
        memcpy(buf[i].iov_base, (iov + i)->iov_base, (iov + i)->iov_len);
        buf[i].iov_len = (iov + i)->iov_len;
        n_bytes_sent += (iov + i)->iov_len;
    }
    pe->pkt.buf = buf;
    pe->pkt.tos = writev_t;
    pe->pkt.sockfd = fd;
    pe->pkt.len = iovcnt;
    
    LL_PREPEND(pkt_list, pe);

    struct sockaddr *dest_addr = get_ip(fd);

    send_has_to_send(dest_addr, pe);

    return n_bytes_sent;
}

/*ssize_t getrandom(void *buf, size_t buflen, unsigned int flags){
    LOGS("getrandom called\n");
    //use get_random to fill a buffer of at least buflen with random bytes then truncate it to buflen
    int n_bytes = 0;
    while (n_bytes < buflen)
    {
        int r = get_random();
        size_t l = sizeof(r) < buflen - n_bytes ? sizeof(r) : buflen - n_bytes;
        memcpy(buf + n_bytes, &r, l);
        n_bytes += l;
    }
    return buflen;
}

int rand(void)
{
    LOGS("gettimeofday called\n");
    return get_random();
}

void srand(unsigned int local_seed)
{
    LOGS("srand called\n");
    seed = local_seed;
    random_number = get_random();
}*/

unsigned int sleep(unsigned int seconds)
{
    LOGS("sleep called\n");
    if (seconds == 0)
        return 0;
    uint64_t start = get_u64_time();
    uint64_t end = start;
    end += seconds*1000000;
    add_event_t(end);
    while (blocking_t() < end)
        ;
    // libc: Zero if the requested time has elapsed,
    //   or the number of seconds left to sleep, if the call was  interrupted
    //   by a signal handler.
    //   We do not currently support signals
    // TODO: see what can be done as we partially support signals now
    return 0;
}

int usleep(useconds_t usec)
{
    LOGS("usleep called\n");
    if (usec == 0)
        return 0;
    uint64_t start = get_u64_time();
    uint64_t end = start;
    end += usec;
    add_event_t(end);
    while (blocking_t() < end) {
        LOGS("in loop, current time: %lu, deadline: %lu\n", get_u64_time(), end);
    }
    return 0;
}

int nanosleep(const struct timespec *duration,
    struct timespec * rem)
{
    LOGS("nanosleep called\n");
    uint64_t t = timeval_to_uint_us(timespec_to_timeval(*duration));
    if (t == 0)
        return 0;
    uint64_t start = get_u64_time();
    uint64_t end = start;
    end += t;
    add_event_t(end);
    while (blocking_t() < end) {
        LOGS("in loop, current time: %lu, deadline: %lu\n", get_u64_time(), end);
    }
    return 0;
}