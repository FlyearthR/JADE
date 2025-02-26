#include <sys/select.h>
#include <sys/time.h>
#include <sys/types.h>
#include <poll.h>
#include <dlfcn.h>
#include <unistd.h>
#include <arpa/inet.h>
#include <errno.h>
#include <sys/epoll.h>
#include <sys/socket.h>
#include <time.h>
#include "communication.h"

int random_number = 42;

int bind(int sockfd, const struct sockaddr *addr, socklen_t addrlen);

int clock_getres(clockid_t clockid, struct timespec *res);

int clock_gettime(clockid_t clockid, struct timespec *tp);

int clock_settime(clockid_t clockid, const struct timespec *tp);

int cmp_fd_ip_ip(fd_ip_elem *a, fd_ip_elem *b);

int close(int fd);

int connect(int sockfd, const struct sockaddr *addr, socklen_t addrlen);

int empty_fun();

int epoll_wait(int epfd, struct epoll_event *events, int maxevents, int timeout);

struct sockaddr *get_ip(int fd);

ssize_t getrandom(void *buf, size_t buflen, unsigned int flags);

int gettimeofday(struct timeval *restrict tv,
                 void *restrict tz);

struct tm *gmtime_r(const time_t *timep, struct tm *result);

int infinity_poll(struct pollfd *fds, nfds_t nfds, int timeout);

int open(const char *pathname, int flags, ...);

int pause(void);

int poll(struct pollfd *fds, nfds_t nfds, int timeout);

int ppoll(struct pollfd *fds, nfds_t nfds,
          const struct timespec *tmo_p, const sigset_t *sigmask);

int pselect(int nfds, fd_set *restrict readfds,
            fd_set *restrict writefds, fd_set *restrict exceptfds,
            const struct timespec *restrict timeout,
            const sigset_t *restrict sigmask);

int rand(void);

ssize_t read_implem(ssize_t (*func)(int, void *, size_t), int fd, void *buf, size_t count);

ssize_t __read_chk(int fd, void *buf, size_t count);

ssize_t read(int fd, void *buf, size_t count);

ssize_t recvfrom(int sockfd, void *buf, size_t len,
                 int flags, struct sockaddr *src_addr,
                 socklen_t *addrlen);

ssize_t recvmsg(int sockfd, struct msghdr *msg, int flags);

int select(int nfds, fd_set *restrict readfds,
           fd_set *restrict writefds, fd_set *restrict exceptfds,
           struct timeval *restrict timeout);

ssize_t send(int sockfd, const void *buf, size_t len, int flags);

ssize_t sendmsg(int sockfd, const struct msghdr *msg, int flags);

ssize_t sendto(int sockfd, const void *buf, size_t len, int flags,
               const struct sockaddr *dest_addr, socklen_t addrlen);

int socket(int domain, int type, int protocol);

unsigned int sleep(unsigned int seconds);

void srand(unsigned int local_seed);

int usleep(useconds_t usec);

ssize_t write(int fildes, const void *buf, size_t nbyte);

ssize_t writev(int fd, const struct iovec *iov, int iovcnt);