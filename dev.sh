#!/bin/bash

mkdir -p run.tmp/proxy/plugins

velocity_jar="run.tmp/proxy/velocity.jar"
if [ ! -e "$velocity_jar" ]; then
  curl --output "$velocity_jar" "https://api.papermc.io/v2/projects/velocity/versions/3.3.0-SNAPSHOT/builds/436/downloads/velocity-3.3.0-SNAPSHOT-436.jar"
fi

viaversion_jar="run.tmp/proxy/plugins/viaversion.jar"
if [ ! -e "$viaversion_jar" ]; then
  curl --output "$viaversion_jar" "https://hangarcdn.papermc.io/plugins/ViaVersion/ViaVersion/versions/5.7.1-SNAPSHOT%2B897/PAPER/ViaVersion-5.7.1-SNAPSHOT.jar"
fi
viabackwards_jar="run.tmp/proxy/plugins/viabackwards.jar"
if [ ! -e "$viabackwards_jar" ]; then
  curl --output "$viabackwards_jar" "https://hangarcdn.papermc.io/plugins/ViaVersion/ViaBackwards/versions/5.7.1-SNAPSHOT%2B544/PAPER/ViaBackwards-5.7.1-SNAPSHOT.jar"
fi
viarewind_jar="run.tmp/proxy/plugins/viarewind.jar"
if [ ! -e "$viarewind_jar" ]; then
  curl --output "$viarewind_jar" "https://hangarcdn.papermc.io/plugins/ViaVersion/ViaRewind/versions/4.0.13/PAPER/ViaRewind-4.0.13.jar"
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
