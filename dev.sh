#!/bin/bash

mkdir -p run.tmp/proxy

velocity_jar="run.tmp/proxy/velocity.jar"
if [ ! -e "$velocity_jar" ]; then
  curl --output "$velocity_jar" "https://api.papermc.io/v2/projects/velocity/versions/3.3.0-SNAPSHOT/builds/415/downloads/velocity-3.3.0-SNAPSHOT-415.jar"
fi

function build_velocity {
  cd velocity
  gradle build
  mkdir -p ../run.tmp/proxy/plugins
  cp build/libs/*.jar ../run.tmp/proxy/plugins
}


(build_velocity)

cp example_configs/velocity/velocity.toml run.tmp/proxy/velocity.toml
SECRET=$(LC_ALL=C tr -dc 'A-Za-z0-9' < /dev/urandom | head -c 12)
echo "$SECRET" > run.tmp/proxy/forwarding.secret

tmux new-session -d "MINIBIT_FORWARDING_SECRET=$SECRET cargo run -- -c example_configs/velocity/minibit.yml" \; split-window "cd run.tmp/proxy && java -jar ./velocity.jar" \; attach
