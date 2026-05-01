#include <sys/socket.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdio.h>
#include <errno.h>
#include <arpa/inet.h>
#include <string.h>
#include <stdbool.h>
#include <unistd.h>
#include <sys/poll.h>
#include <time.h>
#include <sys/time.h>
#include "miniP.h"
#include "delay.h"

int main(int argc, char* argv[])
{
    char *ip_dst = NULL;
    char opt;
    int port_dst = 0;
    int nb = 0;
    int id = 0;
    while ((opt = getopt(argc, argv, "i:p:o:I:")) != -1) {
        switch (opt) {
        case 'i': ip_dst = optarg; break;
        case 'I': id = atoi(optarg); break;
        case 'p': port_dst = atoi(optarg); break;
	    case 'o': nb = atoi(optarg); break;
        default:
            fprintf(stderr, "Usage: %s -i destination IP -I id -p destination port \n", argv[0]);
            exit(EXIT_FAILURE);
        }
    }
    if (!ip_dst || !port_dst ) {
        fprintf(stderr, "1Usage: %s -i destination IP -p destination port\n", argv[0]);
        exit(EXIT_FAILURE);
    }
    
    int fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (fd <= 0) {
        perror("socket: socket");
        exit(EXIT_FAILURE);
    }
    
    struct sockaddr_in v_dst = {};
    inet_pton(AF_INET, ip_dst, &v_dst.sin_addr.s_addr);
    v_dst.sin_port = htons(port_dst);
    v_dst.sin_family = AF_INET;
    
    struct pollfd fds[1];
    fds[0].fd = fd;
    fds[0].events = POLLIN;
    
    int ret = 1;
    struct timeval tv;
    struct msg buf;
    for (int i = 0 ; i < nb && ret; i++) {
	printf("Sending Ping...\n");
	encode_msg(&buf, id, "ping", 3, 0);
	if (sendto(fd, (void*) &buf, sizeof(struct msg), 0, (struct sockaddr*)&v_dst, sizeof(struct sockaddr_in)) != sizeof(struct msg)) {
	    perror("sendto");
	    exit(EXIT_FAILURE);
        }
	ret = poll(fds, 1, TIMEOUT*1000);
        if (ret == -1) {
	    perror ("poll");
	    exit(EXIT_FAILURE);
	} else if (ret) {
	    recv(fd, (void*) &buf, sizeof(struct msg), 0);
	}
    }
    printf("Server did not respond in time\n");

    return -1;
}
