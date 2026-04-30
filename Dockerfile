ARG image="shadowsim/shadow-ci:ubuntu-24.04-gcc-debug"

FROM $image

RUN apt update
RUN apt install --fix-missing -y wget gawk bison build-essential curl snapd python3 sudo make git cmake openssl pkg-config libssl-dev libcunit1 libcunit1-doc libcunit1-dev libc6-dev python3-pandas python3-matplotlib time

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo install cbindgen

RUN mkdir /network-time-simulator
WORKDIR /network-time-simulator
COPY . .

RUN make all

WORKDIR /network-time-simulator/XP
