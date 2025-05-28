#!/bin/bash

EXE=$1

basic_test() {
    EXE=$1
    CFLAGS=$2
    START_PWD=$(pwd)

    gcc $EXE.c $CFLAGS -o ../../testing/$EXE
    sed "s/%%EXE.%%/$EXE/g" ../../tests/2_followers.toml.template |
        sed "s/%%ARGS.%%//g" > ../../testing/2_followers.toml
    cp ../../tests/2_followers.gml ../../testing/2_followers.gml
    cd ../../testing
    sudo RUST_BACKTRACE=1 ./simulator 2_followers.toml
    if [ $? -ne 0 ]
    then
        sudo ip -all netns del
    fi
    grep "Error" *.out > /dev/null
    if [ $? -eq 0 ]
    then
        echo "$EXE is not intra-process coherent"
        exit 1
    fi
    diff 1.out 2.out > /dev/null
    if [ $? -ne 0 ]
    then
        echo "$EXE is not inter-process coherent"
        exit 2
    fi
    mv -f 1.out 1.out.old
    mv -f 2.out 2.out.old
    sudo RUST_BACKTRACE=1 ./simulator 2_followers.toml
    if [ $? -ne 0 ]
    then
        sudo ip -all netns del
    fi
    diff 1.out 1.out.old > /dev/null
    if [ $? -ne 0 ]
    then
        echo "$EXE is not inter-run coherent"
        exit 2
    fi
    diff 2.out 2.out.old > /dev/null
    if [ $? -ne 0 ]
    then
        echo "$EXE is not inter-run coherent"
        exit 2
    fi
    rm -f 1.out.old 2.out.old $EXE
    cd $START_PWD
}

client_server_test() {
    EXE1=$1
    EXE2=$2
    START_PWD=$(pwd)

    gcc $EXE1.c -o ../../testing/$EXE1
    gcc $EXE2.c -o ../../testing/$EXE2
    cat ../../tests/2_followers.toml.template |
    sed "s/%%EXE1%%/$EXE1/g" |
    sed "s/%%EXE2%%/$EXE2/g" |
    sed "s/%%ARGS.%%//g" > ../../testing/2_followers.toml
    cp ../../tests/2_followers.gml ../../testing/2_followers.gml
    cd ../../testing
    if [ "$$(whoami)" != "root" ]; then \
      sudo RUST_BACKTRACE=1 ./simulator 2_followers.toml;\
      else RUST_BACKTRACE=1 ./simulator 2_followers.toml;\
    fi
    if [ $? -ne 0 ]
    then
        if [ "$$(whoami)" != "root" ]; then \
          sudo ip -all netns del;\
          else ip -all netns del;\
        fi
    fi
    grep "Error" *.out > /dev/null
    if [ $? -eq 0 ]
    then
        echo "$EXE1 or $EXE2 is not intra-process coherent"
        exit 1
    fi
    diff 1.out 2.out > /dev/null
    if [ $? -ne 0 ]
    then
        echo "$EXE1 and $EXE2 are not inter-process coherent"
        exit 2
    fi
    mv -f 1.out 1.out.old
    mv -f 2.out 2.out.old
    sudo RUST_BACKTRACE=1 ./simulator 2_followers.toml
    if [ $? -ne 0 ]
    then
        sudo ip -all netns del
    fi
    diff 1.out 1.out.old > /dev/null
    if [ $? -ne 0 ]
    then
        echo "$EXE1 is not inter-run coherent"
        exit 2
    fi
    diff 2.out 2.out.old > /dev/null
    if [ $? -ne 0 ]
    then
        echo "$EXE2 is not inter-run coherent"
        exit 2
    fi
    rm -f 1.out.old 2.out.old $EXE1 $EXE2
    cd $START_PWD
}


cp -f ../../target/debug/simulator ../../testing/simulator
cp -f ../../target/syscalls/syscalls.so ../../testing/syscalls.so

case $EXE in

  recvmsg)
    # TODO
    ;;

  select)
    # TODO
    ;;

  pselect)
    # TODO
    ;;

  poll)
    # TODO
    ;;

  ppoll)
    # TODO
    ;;

  send)
    # TODO
    ;;

  sendto | recvfrom)
    client_server_test sendto_recvfrom_client sendto_recvfrom_server
    ;;

  sendmsg)
    # TODO
    ;;

  connect)
    # TODO
    ;;

  clock_settime)
    # TODO
    ;;

  read)
    # TODO
    ;;

  __read_chk)
    # TODO
    ;;

  gettimeofday | rand | srand | dev_random | dev_urandom | getentropy | clock_getres | clock_gettime | setitimer)
    basic_test $EXE
    ;;

  RAND_bytes)
    basic_test $EXE "-lcrypto"
    ;;

  sleep | usleep)
    basic_test $EXE
    sed "s/%%EXE.%%/$EXE/g" ../../tests/2_followers.toml.template |
        sed "s/%%ARGS1%%/, \"2\"/g" |
        sed "s/%%ARGS2%%/, \"3\"/g" > ../../testing/2_followers.toml
    cd ../../testing && sudo RUST_BACKTRACE=1 ./simulator 2_followers.toml
    if [ $? -ne 0 ]
    then
        sudo ip -all netns del
    fi
    grep "Error" *.out > /dev/null
    if [ $? -eq 0 ]
    then
        echo "$EXE is not intra-process coherent"
        exit 1
    fi
    ;;

  *)
    echo -n "unknown"
    ;;
esac