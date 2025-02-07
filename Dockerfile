FROM ubuntu:latest

RUN apt update
RUN apt install -y build-essential curl snapd python3 sudo make git cmake openssl pkg-config libssl-dev libcunit1 libcunit1-doc libcunit1-dev
RUN 
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN cargo --version
RUN make --version
RUN cmake --version
RUN cargo install cbindgen

RUN mkdir /network-time-simulator
WORKDIR /network-time-simulator
COPY . .

RUN ls
RUN make all
RUN make installTestQuic


CMD [ "make", "CITest" ]