#! /bin/sh

./server -p 8080 -m 20 -n $1 -l $2 &

for i in $(seq 1 1 $1)
do
    ./client -i 127.0.0.1 -p 8080 -I $i -s 1000 -n 20 &
done
