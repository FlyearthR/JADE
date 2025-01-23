#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <arpa/inet.h>
#include <sys/select.h>

int main()
{
    int server_port = 8080;
    int sockfd;
    struct sockaddr_in server_addr, client_addr;
    fd_set readfds;
    char buffer[10];
    socklen_t addr_len = sizeof(client_addr);

    // Create a UDP socket
    if ((sockfd = socket(AF_INET, SOCK_DGRAM, 0)) < 0) {
        perror("Socket creation failed");
        exit(EXIT_FAILURE);
    }

    // Set up server address
    memset(&server_addr, 0, sizeof(server_addr));
    server_addr.sin_family = AF_INET;
    server_addr.sin_addr.s_addr = INADDR_ANY;
    server_addr.sin_port = htons(server_port);

    // Bind the socket to the port
    if (bind(sockfd, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) {
        perror("Bind failed");
        close(sockfd);
        exit(EXIT_FAILURE);
    }

    int ret1 = recvfrom(sockfd, buffer, sizeof(buffer), 0, (struct sockaddr *)&client_addr, &addr_len);
    if (ret1 < 0) {
        perror("Receiving failed");
        close(sockfd);
        exit(EXIT_FAILURE);
    }
    if (ret1 != 5) {
        printf("Error 1\n");
        exit(-1);
    }
    printf("%s\n", buffer);

    int ret2 = recvfrom(sockfd, buffer, sizeof(buffer), 0, (struct sockaddr *)&client_addr, &addr_len);
    if (ret2 < 0) {
        perror("Receiving failed");
        close(sockfd);
        exit(EXIT_FAILURE);
    }
    if (ret2 != 4) {
        printf("Error 2\n");
        exit(-1);
    }
    printf("%s\n", buffer);

    int ret3 = recvfrom(sockfd, buffer, sizeof(buffer), 0, (struct sockaddr *)&client_addr, &addr_len);
    if (ret3 < 0) {
        perror("Receiving failed");
        close(sockfd);
        exit(EXIT_FAILURE);
    }
    if (ret3 != 3) {
        printf("Error 3\n");
        exit(-1);
    }
    printf("%s\n", buffer);
   
    return 0;
}