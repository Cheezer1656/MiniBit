#!/bin/bash

./configure_servers.sh

function build_velocity {
  cd velocity
  gradle build
  mkdir -p ../run.tmp/proxy/plugins
  cp build/libs/*.jar ../run.tmp/proxy/plugins

  curl --output ../run.tmp/proxy/velocity.jar "https://api.papermc.io/v2/projects/velocity/versions/3.3.0-SNAPSHOT/builds/415/downloads/velocity-3.3.0-SNAPSHOT-415.jar"
}

(build_velocity)

set -a && source run.tmp/.env && set +a
tmux new-session -d "cargo run -- --auto run.tmp" \; split-window "VELOCITY_FORWARDING_SECRET=$FORWARDING_SECRET cd run.tmp/proxy && java -jar ./velocity.jar" \; attach
