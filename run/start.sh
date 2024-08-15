#!/bin/bash

cd $(dirname "$0")

declare -A servers
declare  -i port=25566
for file in ./bin/*; do
    port+=1
    name=$(basename "$file")
    servers["$name"]=$port

    # Check if the subdirectory exists
    if [ -d "./$name" ]; then
        (
            cd "./$name"
            sed -i "s/25565/$port/g" server.json
            sed -i '0,/"connection_mode": 1/s//"connection_mode": 3/' server.json
            echo "Starting $name on port $port"
            "../bin/$name" &
        )
    else
        echo "Subdirectory ./$name does not exist, skipping $name."
    fi
done

result=""
for server in "${!servers[@]}"; do
    result+="$server = \"127.0.0.1:${servers[$server]}\"\n"
done

# Remove the last newline
result=$(echo -e "$result" | sed '$d')

# Escape the newlines
result=${result//$'\n'/\\n}

sed -i "0,/lobby = \"127.0.0.1:25566\"/s//$result/" ./proxy/velocity.toml

cd ./proxy
$JAVA_HOME/bin/java -jar ./velocity.jar &

wait