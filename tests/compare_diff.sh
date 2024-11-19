#! /bin/bash

diff testing/sim.*.sendt.log > /dev/null
if [ $? -eq 0 ]
then
    echo Simulator logs are identical
else
    echo Simulator logs are different
fi

diff testing/sim.*.test.log > /dev/null
if [ $? -eq 0 ]
then
    echo Server logs are identical
else
    echo Server logs are different
fi