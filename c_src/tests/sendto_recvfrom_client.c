#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <arpa/inet.h>

int main()
{
    char *ip_dst = "10.0.0.20";
    int port_dst = 8080;

    int sockfd;
    struct sockaddr_in server_addr;
    socklen_t addr_len = sizeof(server_addr);

    // Create a UDP socket
    if ((sockfd = socket(AF_INET, SOCK_DGRAM, 0)) < 0) {
        perror("Socket creation failed");
        exit(EXIT_FAILURE);
    }

    // Set up server address
    memset(&server_addr, 0, sizeof(server_addr));
    server_addr.sin_family = AF_INET;
    server_addr.sin_port = htons(port_dst);
    server_addr.sin_addr.s_addr = inet_addr(ip_dst);

    // Send message to the server
    char msg[] = "test";
    int ret1 = sendto(sockfd, msg, sizeof(msg)/sizeof(char), 0, (struct sockaddr *)&server_addr, addr_len);
    printf("%s\n", msg);
    int ret2 = sendto(sockfd, msg+1, sizeof(msg)/sizeof(char)-1, 0, (struct sockaddr *)&server_addr, addr_len);
    printf("%s\n", msg+1);
    int ret3 = sendto(sockfd, msg+2, sizeof(msg)/sizeof(char)-2, 0, (struct sockaddr *)&server_addr, addr_len);
    printf("%s\n", msg+2);

    if (ret1 != sizeof(msg)/sizeof(char)) {
        printf("Error 1\n");
        exit(-1);
    }
    if (ret2 != sizeof(msg)/sizeof(char)-1) {
        printf("Error 2\n");
        exit(-1);
    }
    if (ret3 != sizeof(msg)/sizeof(char)-2) {
        printf("Error 3\n");
        exit(-1);
    }
   
    return 0;
}