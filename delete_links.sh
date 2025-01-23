#!/bin/bash
for x in {1..11}; do
    sudo ip link delete veth_10_$x
    sudo ip netns del $x
done