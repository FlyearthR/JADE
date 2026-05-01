#include <poll.h>
#include <time.h>
#include <sys/socket.h>
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include <arpa/inet.h>
#include <string.h>
#include <stdbool.h>
#include <unistd.h>
#include <pthread.h>
#include <sys/time.h>
#include "miniP.h"
#include "delay.h"

#define MAX_CLIENTS 100

typedef struct {
    struct sockaddr_in addr;
    socklen_t addrlen;
    int exchange_count;
    time_t last_seen;
} client_t;

unsigned long long start_time = 0;
int nb = 0;
client_t clients[MAX_CLIENTS];
int num_clients = 0;

int find_client(struct sockaddr_in *addr) {
    for (int i = 0; i < num_clients; i++) {
        if (clients[i].addr.sin_addr.s_addr == addr->sin_addr.s_addr &&
            clients[i].addr.sin_port == addr->sin_port) {
            return i;
        }
    }
    return -1;
}

void add_client(struct sockaddr_in *addr, socklen_t addrlen) {
    if (num_clients < MAX_CLIENTS) {
        memcpy(&clients[num_clients].addr, addr, addrlen);
        clients[num_clients].addrlen = addrlen;
        clients[num_clients].exchange_count = 0;
        clients[num_clients].last_seen = time(NULL);
        printf("New client: %s:%d (total: %d)\n", inet_ntoa(addr->sin_addr), ntohs(addr->sin_port), num_clients + 1);
        num_clients++;
    }
}

int main(int argc, char* argv[])
{
    char *ip = NULL;
    int port = 0;
    char opt;
    while ((opt = getopt(argc, argv, "i:p:o:")) != -1) {
        switch (opt) {
        case 'i': ip = optarg; break;
        case 'p': port = atoi(optarg); break;
	case 'o': nb = atoi(optarg); break;
        default:
            fprintf(stderr, "Usage: %s -i IP -p port -o nb_exchanges\n", argv[0]);
            exit(EXIT_FAILURE);
        }
    }

    if (!ip || !port || !nb) {
	fprintf(stderr, "Usage: %s -i IP -p port -o nb_exchanges\n", argv[0]);
        exit(EXIT_FAILURE);
    }

    int fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (fd <= 0) {
        perror("socket");
        exit(EXIT_FAILURE);
    }

    struct sockaddr_in v_dst = {};
    inet_pton(AF_INET, ip, &v_dst.sin_addr.s_addr);
    v_dst.sin_port = htons(port);
    v_dst.sin_family = AF_INET;

    if (bind(fd, (struct sockaddr*) &v_dst, sizeof(struct sockaddr_in)) != 0) {
        char s[100];
        sprintf(s, "bind to addr %s", ip);
        perror(s);
        exit(EXIT_FAILURE);
    }

    printf("Server listening on %s:%d\n", ip, port);
    printf("Will send %d exchanges per client\n", nb);

    struct msg buf;
    struct sockaddr_in from;
    socklen_t fromlen;
    struct pollfd fds[1];
    fds[0].fd = fd;
    fds[0].events = POLLIN;

    while (1) {
        int ret = poll(fds, 1, TIMEOUT * 1000);

        if (ret == -1) {
            perror("poll");
            exit(EXIT_FAILURE);
        } else if (ret == 0) {
            // Timeout
            continue;
        }

        if (fds[0].revents & POLLIN) {
            fromlen = sizeof(from);
            if (recvfrom(fd, (void*) &buf, sizeof(struct msg), 0, (struct sockaddr*) &from, &fromlen) == -1) {
                perror("recvfrom");
                continue;
            }

            printf("Message from %s:%d - ", inet_ntoa(from.sin_addr), ntohs(from.sin_port));
            print_msg(buf);

            int client_idx = find_client(&from);
            if (client_idx == -1) {
                add_client(&from, fromlen);
                client_idx = num_clients - 1;
            }

            clients[client_idx].exchange_count++;
            clients[client_idx].last_seen = time(NULL);

            // Send pong response if client hasn't completed exchanges
            if (clients[client_idx].exchange_count <= nb) {
                encode_msg(&buf, 2, "pong", 3, 0);
                if (sendto(fd, (void*) &buf, sizeof(struct msg), 0,
                          (struct sockaddr*) &from, fromlen) != sizeof(struct msg)) {
                    perror("sendto");
                }
                printf("Sent Pong to %s:%d (exchange %d/%d)\n",
                       inet_ntoa(from.sin_addr), ntohs(from.sin_port),
                       clients[client_idx].exchange_count, nb);
            }
        }
    }

    close(fd);
    return 0;
}

