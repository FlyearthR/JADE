#!/bin/bash

cd ../examples/picoquic
# ./picoquicdemo  -q ../logs -o ../recieved_pages_quic localhost 4433 index.html
./picoquicdemo  -q ../logs -n servername -o ../recieved_pages_quic/ 10.0.0.20 4433 index.html
# ./picoquicdemo -o ../recieved_pages_quic/ localhost 4433 static.html
# ./picoquicdemo -o ../recieved_pages_quic/ localhost 4433 dynamic.html
# ./picoquicdemo -o ../recieved_pages_quic/ localhost 4433 form.html
# ./picoquicdemo -o ../recieved_pages_quic/ localhost 4433 images.html
# ./picoquicdemo -o ../recieved_pages_quic/ localhost 4433 heavy.html
# ./picoquic_sample client 10.0.0.20 4433  test_folder/ static.html 
# ./picoquic_sample client localhost 4433  test_folder/ static.html 
# ./picoquicdemo -o ../received_pages_quic/ 10.0.0.20 4433 images.html
# ./picoquicdemo -o . localhost 4433 images.html
cd ../..