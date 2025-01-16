#include <sys/select.h>
#include <poll.h>
#include <sys/time.h>
#include <sys/types.h>
#include <dlfcn.h>
#include <unistd.h>
#include <arpa/inet.h>
#include <sys/socket.h>
#include "communication.h"

int random_number = 42;

int empty_fun()
{
        //printf("empty_fun called\n");
        blocking();
        return 1;
}

ssize_t recvfrom(int sockfd, void* buf, size_t len,
                        int flags, struct sockaddr * src_addr,
                        socklen_t * addrlen)
{
        // TODO: configure socket as non blocking at opening time
        //printf("inside recvfrom\n");
        // fflush(stdout);
        int flags_s = fcntl(sockfd, F_GETFL, 0);
        if (flags_s == -1)
                return -1;
        //printf("inside recvfrom 2\n");
        // fflush(stdout);
        fcntl(sockfd, F_SETFL, O_NONBLOCK);
        LIBC_FUNCTION(ssize_t, recvfrom, int sockfd, void* buf, size_t len,
                        int flags, struct sockaddr * src_addr,
                        socklen_t * addrlen);
        //printf("inside recvfrom 3\n");
        // fflush(stdout);
        int ret;
        do {
        printf("inside recvfrom loop\n");
        fflush(stdout);
                ret = LIBC_FUNCTION_GET(recvfrom)(sockfd, buf, len, flags, src_addr, addrlen);
                perror("recvfrom: ");
                fflush(stderr);
        } while (ret == -1 && empty_fun());
        return ret;
}

ssize_t recvmsg(int sockfd, struct msghdr *msg, int flags){
        int flags_s = fcntl(sockfd, F_GETFL, 0);
        if (flags_s == -1)
                return -1;
        fcntl(sockfd, F_SETFL, O_NONBLOCK);
        LIBC_FUNCTION(ssize_t, recvmsg, int sockfd, struct msghdr *msg, int flags);
        int ret;
        while (ret == -1 && empty_fun()){
                ret = LIBC_FUNCTION_GET(recvmsg)(sockfd, msg, flags);
        }
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
        //printf("select called\n");
	cur = blocking();
	while (ret == 0 && before_timeval(cur, to)) {
		ret = LIBC_FUNCTION_GET(select)(nfds, readfds, writefds, exceptfds, &zeros);
        //printf("select loop called\n");
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
        //printf("pselect called\n");
        cur = blocking();
	while (ret == 0 && before_timeval(cur, to)) {
                ret = LIBC_FUNCTION_GET(pselect)(nfds, readfds, writefds, exceptfds, &zeros, sigmask);
        //printf("pselect loop called\n");
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
        //printf("poll called\n");
        cur = blocking();
	do {
                ret = LIBC_FUNCTION_GET(poll)(fds, nfds, 0);
        //printf("poll loop called\n");
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
        //printf("ppoll called\n");
        cur = blocking();
	do {
                ret = LIBC_FUNCTION_GET(ppoll)(fds, nfds, &zeros, sigmask);
        //printf("ppoll loop called\n");
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
        if (seconds == 0)
                return 0;
        struct timeval start = get_time();
        struct timeval end = start;
        end.tv_sec += seconds;
        add_event(end);
        //printf("sleep called (multiple blocking possible)\n");
        while(before_timeval(blocking(), end));
        // libc: Zero if the requested time has elapsed,
        //   or the number of seconds left to sleep, if the call was  interrupted
        //   by a signal handler.
        //   We do not currently support signals
        //
        return 0;
}

int usleep(useconds_t usec)
{
        if (usec == 0)
                return 0;
        struct timeval start = get_time();
        struct timeval end = {.tv_sec = 0, .tv_usec = usec};
        end = add_timeval(start, end);
        add_event(end);
        //printf("usleep called (multiple blocking possible)\n");
        while(before_timeval(blocking(), end));
        return 0;
}

void srand(unsigned int local_seed)
{
        fflush(stdout);
        seed=local_seed;
        random_number = get_random();
}

int rand(void)
{
        return get_random();
}

/*ssize_t send(int sockfd, const void* buf, size_t len, int flags)
{
        packet_elem* pe;
        if ((pe = (packet_elem*)malloc(sizeof *pe)) == NULL) exit(-13);
        if ((pe->pkt.buf = malloc(len)) == NULL) exit(-13);

        pe->pkt.id = next_pkt_id++;
        pe->pkt.tos = send_t;
        pe->pkt.sockfd = sockfd;
        memcpy((void*) pe->pkt.buf, buf, len);
        pe->pkt.len = len;
        pe->pkt.flags = flags;
        pe->pkt.dest_addr = NULL;
        pe->pkt.addrlen = 0;

        LL_PREPEND(pkt_list, pe);

        // struct sockaddr* dest_addr = get_ip(sockfd);

        struct sockaddr_in dest_addr;
        bzero(&dest_addr, sizeof(dest_addr));
        socklen_t addrlen = sizeof(dest_addr);
        if (getsockname(sockfd, &dest_addr, &addrlen) == -1) exit(-13);

        send_has_to_send(&dest_addr, pe);

        return len;
}*/

ssize_t sendto(int sockfd, const void* buf, size_t len, int flags,
                      const struct sockaddr *dest_addr, socklen_t addrlen)
{
        //printf("sendto intercepted\n");
        // fflush(stdout);
        packet_elem* pe;
        if ((pe = (packet_elem*)malloc(sizeof *pe)) == NULL) exit(-13);
        if ((pe->pkt.buf = malloc(len)) == NULL) exit(-13);

        pe->pkt.id = next_pkt_id++;
        pe->pkt.tos = sendto_t;
        pe->pkt.sockfd = sockfd;
        memcpy((void*) pe->pkt.buf, buf, len);
        pe->pkt.len = len;
        pe->pkt.flags = flags;
        if ((pe->pkt.dest_addr = (struct sockaddr*)malloc(addrlen)) == NULL) exit(-13);
        memcpy((void*) pe->pkt.dest_addr, dest_addr, addrlen);
        pe->pkt.addrlen = addrlen;

        LL_PREPEND(pkt_list, pe);
        // printf("packet prepended\n");
        // fflush(stdout);

        send_has_to_send(dest_addr, pe);
//         switch(dest_addr->sa_family) {
//         case AF_INET:
//             char* ip = (char*) (&((struct sockaddr_in*)dest_addr)->sin_addr.s_addr);
//             Message m = {
//                 .tag = HasToSend4,
//                 .has_to_send4 = {
//                         ._0 = ID,
//                         ._1 = 0, // TODO: find interface id
//                         ._2 = {.segments = {ip[0], ip[1], ip[2], ip[3]}},
//                         ._3 = pe->pkt.id
//                         }
//                 };
//             send_msg(m);
//             break;

//         case AF_INET6:
//             Message m2 = {
//                 .tag = HasToSend6,
//                 .has_to_send6 = {
//                         ._0 = ID,
//                         ._1 = 0, // TODO: find interface id
//                         ._2 = {
//                                 .segments = {
//                                         ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[0],
//                                         ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[1],
//                                         ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[2],
//                                         ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[3],
//                                         ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[4],
//                                         ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[5],
//                                         ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[6],
//                                         ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[7]
//                                         }
//                                 },
//                         ._3 = pe->pkt.id
//                         }
//                 };
//             send_msg(m2);
//             break;

//         default:
//             fprintf(stderr, "Unknown AF\n");
//             return 0;
//     }

        //printf("sendto interception end\n");
        //fflush(stdout);
        return len;
}

/*ssize_t sendmsg(int sockfd, const struct msghdr *msg, int flags){
        //TODO
        packet_elem* pe;
        if ((pe = (packet_elem*)malloc(sizeof *pe)) == NULL) exit(-13);
        if ((pe->pkt.buf = malloc(sizeof(struct msghdr))) == NULL) exit(-13);
        struct msghdr* buf = (struct msghdr*) pe->pkt.buf;
        buf->msg_name = malloc(msg->msg_namelen);
        memcpy(buf->msg_name,msg->msg_name,msg->msg_namelen);
        buf->msg_namelen = buf->msg_namelen;
        buf->msg_iov = (struct iovec*) malloc(sizeof(struct iovec)*msg->msg_iovlen);
        int n_bytes_sent = 0;
        for (int i = 0; i<msg->msg_iovlen; i++){
                size_t len = (msg->msg_iov +i)->iov_len;
                n_bytes_sent += len;
                void* tmp_buf = malloc(len);
                memcpy(tmp_buf,(msg->msg_iov+i)->iov_base, len);
                struct iovec* iov = (struct iovec*) buf->msg_iov;
                iov += i;
                iov->iov_base = buf;
                iov->iov_len = len;
        }
        buf->msg_control = malloc(msg->msg_controllen);
        memcpy(buf->msg_control, msg->msg_control,msg->msg_controllen);
        buf->msg_controllen = msg->msg_controllen;
        buf->msg_flags=msg->msg_flags;
        pe->pkt.id = next_pkt_id++;
        pe->pkt.tos = sendmsg_t;
        pe->pkt.sockfd = sockfd;
        pe->pkt.len = sizeof(struct msghdr);
        pe->pkt.flags = flags;
        pe->pkt.dest_addr = NULL;
        pe->pkt.addrlen=0;

        LL_PREPEND(pkt_list,pe);

        // struct sockaddr* dest_addr = get_ip(sockfd);

        
        // struct sockaddr* dest_addr = get_ip(sockfd);
        // struct sockaddr *dest_addr = (struct sockaddr*) malloc(sizeof(struct sockaddr));
        // socklen_t addrlen = sizeof(struct sockaddr)+10;
        // printf("addrlen 1 %d\n", addrlen);
        // Get my ip address and port
        struct sockaddr_in dest_addr;
        bzero(&dest_addr, sizeof(dest_addr));
        socklen_t addrlen = sizeof(dest_addr);
        if (getsockname(sockfd, &dest_addr, &addrlen) == -1) exit(-13);

        pe->pkt.dest_addr = &dest_addr;
        pe->pkt.addrlen=addrlen;
        printf("addrlen 2 %d", addrlen);

        // printf("dest addr : 0x%02X", dest_addr->sa_data);
        printf("sockaddr :\n");
        char *toprint = (char *)&dest_addr;
        for (int i =0; i<addrlen;i++){
                printf("%02X", toprint[i]);
        }
         
        send_has_to_send(&dest_addr, pe);

        return n_bytes_sent;
}*/

void send_has_to_send(const struct sockaddr *dest_addr, packet_elem* pe){
        switch(dest_addr->sa_family) {
        case AF_INET:
            char* ip = (char*) (&((struct sockaddr_in*)dest_addr)->sin_addr.s_addr);
            Message m = {
                .tag = HasToSend4,
                .has_to_send4 = {
                        ._0 = ID,
                        ._1 = 0, // TODO: find interface id
                        ._2 = {.segments = {ip[0], ip[1], ip[2], ip[3]}},
                        ._3 = pe->pkt.id
                        }
                };
            send_msg(m);
            break;

        case AF_INET6:
            Message m2 = {
                .tag = HasToSend6,
                .has_to_send6 = {
                        ._0 = ID,
                        ._1 = 0, // TODO: find interface id
                        ._2 = {
                                .segments = {
                                        ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[0],
                                        ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[1],
                                        ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[2],
                                        ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[3],
                                        ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[4],
                                        ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[5],
                                        ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[6],
                                        ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[7]
                                        }
                                },
                        ._3 = pe->pkt.id
                        }
                };
            send_msg(m2);
            break;

        default:
            fprintf(stderr, "Unknown AF\n");
            return 0;
    }
}

// ssize_t read(int fildes, void *buf, size_t nbyte){
        //TODO
// }

// ssize_t write(int fildes, const void *buf, size_t nbyte){
//         TODO
// }

// off_t lseek(int fildes, off_t offset, int whence){
//         TODO
// }

int connect(int sockfd, const struct sockaddr *addr,
                   socklen_t addrlen)
{
        fd_ip_elem* f = (fd_ip_elem*) malloc(sizeof(fd_ip_elem));
        f->fi.fd = sockfd;
        memcpy(&f->fi.addr, addr, addrlen);
        LL_PREPEND(fd_ip_list, f);
        
        LIBC_FUNCTION(int, connect, int sockfd, const struct sockaddr *addr,
                   socklen_t addrlen);
	return LIBC_FUNCTION_GET(connect)(sockfd, addr, addrlen);
}