#!/bin/bash

cd ../examples/picoquic
# ./picoquicdemo -p 4433 -w ../web_page_quic/
# ./picoquic_sample server 4433 certs/test-ca.crt certs/key.pem test_folder/
./picoquicdemo -p 4433 -w ../web_page_quic/ -1 -q ../logs
cd ../..