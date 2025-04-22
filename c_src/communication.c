#include "communication.h"
#include <dlfcn.h>

// TODO: fix memory leaks due to (de)serialization

int id = 0;
int seed = 0;
int fdi = 0;
int fdo = 0;
uint64_t current_time;

void __attribute__((constructor)) init_fd()
{ // TODO: get the name of the queue from the env
    id = atoi(getenv("ID"));
    char out[10] = "";
    strcat(out, getenv("ID"));
    strcat(out, ".out");
    freopen(out, "w", stdout);
    char err[10] = "";
    strcat(err, getenv("ID"));
    strcat(err, ".err");
    freopen(err, "w", stderr);

    char FD[20] = "/some_queue_";
    strcat(FD, getenv("ID"));
    struct mq_attr attr = {
        .mq_flags = 0,
        .mq_maxmsg = 20,
        .mq_msgsize = 27,
        .mq_curmsgs = 0,
    };
    fdi = mq_open(FD, O_RDONLY | O_CREAT, (mode_t)0600, attr);

    FD[12] = '0';
    FD[13] = '\0';
    fdo = mq_open(FD, O_WRONLY | O_CREAT, (mode_t)0600, attr);

    pkt_list = NULL;
    fd_ip_list = NULL;
    next_pkt_id = 0;

    char path[100];

    snprintf(path, 100, "%snode%i.log", "../logs/", id);

    log_file = fopen(path, "w"); // TODO: get log file from the env

    LOGS("Process %i properly preloaded\n", id);

    current_time = receive_msg();
}

void print_msg(Message m)
{
    switch (m.tag)
    {
    case Stuck:
        LOGS("Stuck(%i)\n", m.stuck);
        break;
    case AddStep:
        LOGS("AddStep(%i, %li)\n", m.add_step._0, m.add_step._1);
        break;
    case DelStep:
        LOGS("DelStep(%i, %li)\n", m.del_step._0, m.del_step._1);
        break;
    case HasToSend4:
        LOGS("HasToSend4(%i, %i, %i.%i.%i.%i, %li)\n", m.has_to_send4._0, m.has_to_send4._1,
             m.has_to_send4._2.segments[0], m.has_to_send4._2.segments[1], m.has_to_send4._2.segments[2],
             m.has_to_send4._2.segments[3], m.has_to_send4._3);
        break;
    case HasToSend6:
        LOGS("HasToSend6(%i, %i, %i:%i:%i:%i:%i:%i:%i:%i, %li)\n", m.has_to_send6._0, m.has_to_send6._1,
             m.has_to_send6._2.segments[0], m.has_to_send6._2.segments[1], m.has_to_send6._2.segments[2],
             m.has_to_send6._2.segments[3], m.has_to_send6._2.segments[4], m.has_to_send6._2.segments[5],
             m.has_to_send6._2.segments[6], m.has_to_send6._2.segments[7], m.has_to_send6._3);
        break;
    case Send:
        LOGS("Send(%li)\n", m.send);
        break;
    case Sent:
        LOGS("Sent(%i, %li)\n", m.sent._0, m.sent._1);
        break;
    case GetTime:
        LOGS("GetTime(%i)\n", m.get_time);
        break;
    case GetRand:
        LOGS("GetRand(%li)\n", m.get_rand);
        break;
    case WakeUp:
        LOGS("WakeUp(%li)\n", m.wake_up);
        break;
    case Finished:
        LOGS("Finished(%i)\n", m.finished);
        break;
    }
}

int send_msg(Message m) // TODO: extend this
{
    LOGS("inside send_msg\n");
    print_msg(m);
    Buffer *b = serialize(m);
    int ret = mq_send(FDO, b->buffer, SIZE_BUFFER, (m.tag == GetTime || m.tag == GetRand) ? 2 : 1);
    return ret;
}

uint64_t receive_msg()
{
    LOGS("receive_msg\n");
    Buffer msg;
    do
    {
        int ret = mq_receive(FDI, msg.buffer, SIZE_BUFFER, NULL);
        if (ret == -1)
        {
            perror("Error: ");
            exit(-6);
        }
        LOGS("received a message\n");
        for (int i = 0; i < SIZE_BUFFER; i++)
        {
            LOGS("%3i ", msg.buffer[i]);
        }
        LOGS("\n");
        Message *m = deserialize(msg);
        print_msg(*m);
        if (m->tag == Send)
        {
            sender(m->send);
        }
        else
        {
            current_time = m->wake_up;
            // check if we have to send a signal
            if (before_timeval(timer_real.it_value, u64_to_timeval_us(current_time))) {
                raise(SIGALRM);
                timer_real.it_value = add_timeval(u64_to_timeval_us(current_time), timer_real.it_interval);
                // we could have also received a message, so we should leave the loop
            }
            return m->wake_up;
        }
    } while (true);
}

struct timeval get_time()
{
    return u64_to_timeval_us(current_time);
}

uint64_t get_u64_time()
{
    return current_time;
}

struct timeval blocking()
{
    Message m = {.tag = Stuck, .stuck = ID};
    unsigned int ret = send_msg(m);
    if (ret != 0)
        exit(-8);
    ret = receive_msg();

    return u64_to_timeval_us(ret);
}

void add_event(struct timeval t)
{
    Message m = {.tag = AddStep, .add_step = {._0 = ID, ._1 = timeval_to_uint_us(t)}};
    unsigned int ret = send_msg(m);
    if (ret != 0)
        exit(-9);
}

void suppress_event(struct timeval t)
{
    Message m = {.tag = DelStep, .del_step = {._0 = ID, ._1 = timeval_to_uint_us(t)}};
    print_msg(m);
    unsigned int ret = send_msg(m);
    if (ret != 0)
        exit(-10);
}

int get_random()
{
    Message m = {.tag = GetRand, .get_rand = {._0 = ID, ._1 = seed}};
    unsigned int ret = send_msg(m);
    if (ret != 0)
        exit(-11);
    return receive_msg();
}

void custom_send(packet pkt)
{
    LIBC_FUNCTION(ssize_t, send, int sockfd, const void *buf, size_t len, int flags);
    int ret = LIBC_FUNCTION_GET(send)(pkt.sockfd, pkt.buf, pkt.len, pkt.flags);
    free((void *)pkt.buf);
}

void custom_sendto(packet pkt)
{

    LIBC_FUNCTION(ssize_t, sendto, int sockfd, const void *buf, size_t len, int flags, const struct sockaddr *dest_addr, socklen_t addrlen);
    int ret = LIBC_FUNCTION_GET(sendto)(pkt.sockfd, pkt.buf, pkt.len, pkt.flags, pkt.dest_addr, pkt.addrlen);
    free((void *)pkt.dest_addr);
    free((void *)pkt.buf);
}

void custom_sendmsg(packet pkt)
{
    LOGS("custom_sendmsg\n");
    LIBC_FUNCTION(ssize_t, sendmsg, int sockfd, const struct msghdr *msg, int flags);
    int ret = LIBC_FUNCTION_GET(sendmsg)(pkt.sockfd, (struct msghdr *)pkt.buf, pkt.flags);
    if (ret)
        perror("sendmsg: ");
    LOGS("return value custom_sendmsg : %d\n", ret);

    struct msghdr *buf = (struct msghdr *)pkt.buf;
    for (int i = 0; i < buf->msg_iovlen; i++)
    {
        free((buf->msg_iov + i)->iov_base);
    }
    free(buf->msg_iov);
    free(buf->msg_control);
    free(buf->msg_name);
    free((void *)pkt.buf);
}

void custom__write(packet pkt)
{
    LIBC_FUNCTION(ssize_t, write, int fildes, const void *buf, size_t nbyte);
    int ret = LIBC_FUNCTION_GET(write)(pkt.sockfd, pkt.buf, pkt.len);
    free((void *)pkt.buf);
}

void custom__writev(packet pkt)
{
    LIBC_FUNCTION(ssize_t, writev, int fd, const struct iovec *iov, int iovcnt);
    int ret = LIBC_FUNCTION_GET(writev)(pkt.sockfd, (struct iovec *)pkt.buf, pkt.len);
    struct iovec *buf = (struct iovec *)pkt.buf;
    for (int i = 0; i < pkt.len; i++)
    {
        free(buf[i].iov_base);
    }
    free((void *)pkt.buf);
}

void sender(uint64_t pkt_id)
{
    LOGS("sender called\n");
    packet_elem goal = {.pkt = {.id = pkt_id}};
    packet_elem *found = NULL;
    LL_SEARCH(pkt_list, found, &goal, cmp_pkt);
    if (!found)
        exit(-12);
    LOGS("pkt tos : %d\n", found->pkt.tos);
    switch (found->pkt.tos)
    {
    case send_t:
        custom_send(found->pkt);
        break;

    case sendto_t:
        custom_sendto(found->pkt);
        break;

    case sendmsg_t:
        custom_sendmsg(found->pkt);
        break;

    case write_t:
        custom__write(found->pkt);
        break;
    
    case writev_t:
        custom__writev(found->pkt);
        break;
    }

    LL_DELETE(pkt_list, found);
    free(found);
    Message m = {.tag = Sent, .sent = {._0 = ID, ._1 = pkt_id}};
    send_msg(m);
}

void send_has_to_send(const struct sockaddr *dest_addr, packet_elem *pe)
{
    LOGS("send_has_to_send called\n");
    LOGS("sa family %d\n", dest_addr->sa_family);
    switch (dest_addr->sa_family)
    {
    case AF_INET:
        LOGS("HasToSend4\n");
        char *ip = (char *)(&((struct sockaddr_in *)dest_addr)->sin_addr.s_addr);
        Message m = {
            .tag = HasToSend4,
            .has_to_send4 = {
                ._0 = ID,
                ._1 = 0, // TODO: find interface id
                ._2 = {.segments = {ip[0], ip[1], ip[2], ip[3]}},
                ._3 = pe->pkt.id}};
        send_msg(m);
        break;

    case AF_INET6:
        LOGS("HasToSend6\n");
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
                        ((struct sockaddr_in6 *)dest_addr)->sin6_addr.__in6_u.__u6_addr16[7]}},
                ._3 = pe->pkt.id}};
        send_msg(m2);
        break;

    default:
        fprintf(stderr, "Unknown AF\n");
    }
}

void __attribute__((destructor)) send_finished()
{
    int count;
    packet_elem *p;
    Buffer msg;
    LL_COUNT(pkt_list, p, count);
    if (count != 0) // if there are still messages to send
    {
        Message m_stuck = {.tag = Stuck, .stuck = ID};
        send_msg(m_stuck);
        while (count)
        {
            LOGS("Finishing loop count: %i\n", count);
            int ret = mq_receive(FDI, msg.buffer, SIZE_BUFFER, NULL);
            if (ret == -1)
            {
                fprintf(stderr, "Error receiving message in send_finished\n");
                perror("Error: ");
                exit(-6);
            }
            Message *m = deserialize(msg);
            print_msg(*m);
            if (m->tag == Send)
            {
                LOGS("FL: Send\n");
                sender(m->send);
                count--;
            }
            else if (m->tag == WakeUp)
            {
                send_msg(m_stuck);
            }
            else
            {
                // ignore
            }
        }
    }
    Message m = {.tag = Finished, .finished = ID};
    send_msg(m);
}