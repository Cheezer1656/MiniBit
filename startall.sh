#!/bin/bash

mkdir -p out/bin
cp -r run/* out/

out/configure.sh

(cd gate && go build -ldflags "-s -w")

tmux new-session -d "cargo run -- --auto out" \; split-window "GATE_VELOCITY_SECRET=$(<out/proxy/forwarding.secret) gate/gate -c out/proxy/config.yml" \; attach
