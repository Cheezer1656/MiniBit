#!/bin/bash

cargo build -r

function build_velocity() {
  cd velocity
  gradle build
}

mkdir -p out/bin
cp -r run/* out/
find target/release -perm u=rwx,g=rx,o=rx -type f -maxdepth 1 -exec cp {} out/bin \;

(build_velocity)
mkdir -p out/proxy/plugins
cp velocity/build/libs/* out/proxy/plugins/
curl --output out/proxy/velocity.jar "https://api.papermc.io/v2/projects/velocity/versions/3.3.0-SNAPSHOT/builds/415/downloads/velocity-3.3.0-SNAPSHOT-415.jar"
