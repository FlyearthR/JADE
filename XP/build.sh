#! /bin/bash
git clone https://github.com/shadow/shadow.git
cd shadow
git checkout d208326cf43393f32289d28e35d42d4cca96ff8e
./ci/run.sh -i 1 || true
cd ..
docker build -t jade --build-arg image=shadowsim/shadow-ci:ubuntu-24.04-gcc-debug .
