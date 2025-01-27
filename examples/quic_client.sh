#!/bin/bash

cd ../examples/picoquic
# ./picoquicdemo  -q ../logs -n servername -o ../received_pages_quic localhost 4433 index.html
echo "cd done"
./picoquicdemo  -q ../logs -n servername -o ../received_pages_quic/ 10.0.0.20 4433 index.html
# ./picoquicdemo -o ../received_pages_quic/ localhost 4433 static.html
# ./picoquicdemo -o ../received_pages_quic/ localhost 4433 dynamic.html
# ./picoquicdemo -o ../received_pages_quic/ localhost 4433 form.html
# ./picoquicdemo -o ../received_pages_quic/ localhost 4433 images.html
# ./picoquicdemo -o ../received_pages_quic/ localhost 4433 heavy.html
# ./picoquic_sample client 10.0.0.20 4433  test_folder/ static.html 
# ./picoquic_sample client localhost 4433  test_folder/ static.html 
# ./picoquicdemo -o ../received_pages_quic/ 10.0.0.20 4433 images.html
# ./picoquicdemo -o . localhost 4433 images.html
cd ../..