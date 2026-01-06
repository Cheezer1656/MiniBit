#!/bin/bash

./configure_servers.sh

(cd gate && go build -ldflags "-s -w")

set -a && source run.tmp/.env && set +a
tmux new-session -d "cargo run -- --auto run.tmp" \; split-window "GATE_VELOCITY_SECRET=$FORWARDING_SECRET gate/gate -c run.tmp/proxy/config.yml" \; attach
