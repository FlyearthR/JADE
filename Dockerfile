ARG image

FROM ubuntu:latest

RUN apt update
RUN apt install --fix-missing -y wget gawk bison build-essential curl snapd python3 sudo make git cmake openssl pkg-config libssl-dev libcunit1 libcunit1-doc libcunit1-dev libc6-dev

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install cbindgen

RUN mkdir /network-time-simulator
WORKDIR /network-time-simulator
COPY . .

WORKDIR /network-time-simulator

RUN make static

ARG image

FROM $image:latest

COPY --from=0 /network-time-simulator /network-time-simulator
