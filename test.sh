#!/bin/bash

if [ "$EUID" -ne 0 ]
  then echo "Please run as root"
  exit -1
fi

cargo test --package network_time_simulator --bin simulator -- unit_testing --show-output --nocapture
if [ $? -ne 0 ]
  then echo "Unit test failing"
  exit -1
fi

cargo test --package network_time_simulator --bin simulator -- determinism --show-output --nocapture
if [ $? -ne 0 ]
  then echo "Determinism test failing"
  exit -1
fi