#! /bin/bash

EXE=$1

file $EXE | grep -q "dynamically linked"
if [ $? -eq 1 ]
then
    echo "Your executable does not seem dynamically linked, you need to link dynamically the libc"
    exit -1
fi

objdump -T $EXE | grep GLIBC | sed 's/ [a-z] /   /g' | awk '{print $6}' > /tmp/libc_calls

if [ "$(wc -c < /tmp/libc_calls)" -eq "0" ]
then
    echo "Your executable does not seem to call the libc"
    exit -1
fi

cat implemented not_needed > /tmp/libc_calls_ok

if grep -Fxq /tmp/libc_calls /tmp/libc_calls_ok
then
    echo "Your executable should be supported by NTS"
    exit 0
fi

CALLS=$(cat /tmp/libc_calls)

for CALL in $CALLS
do
    if grep -q "\<$CALL\>" partially_implemented
    then
        echo "$CALL: is partially implemented, you should check if your desired behaviour are supported"
    elif grep -q "\<$CALL\>" todo
    then
        echo "$CALL: is not implemented and should be"
    elif grep -q "\<$CALL\>" implemented
    then
        #ok
        :
    elif grep -q "\<$CALL\>" not_needed
    then
        #ok
        :
    else
        echo "$CALL: is not known by the system"
    fi
done