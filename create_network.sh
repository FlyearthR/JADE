#!/usr/bin/env bash
ip netns add 1
ip netns add 10

ip link add 1-10 type veth peer name 10-1

ip link set 1-10 netns 1
ip link set 10-1 netns 10

ip netns exec 10 ip addr add 10.0.0.20/24 dev 10-1
ip netns exec 1 ip addr add 10.0.0.1/24 dev 1-10

ip netns exec 10 ip link set 10-1 up
ip netns exec 1 ip link set 1-10 up