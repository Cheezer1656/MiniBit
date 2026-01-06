#!/bin/bash

cd $(dirname "$0")

result=""
declare -i port=25566
for file in ./bin/*; do
    name=$(basename "$file")

    # Check if the subdirectory exists
    if [ -d "./$name" ]; then
        port+=1
        result+="$name = \"127.0.0.1:$port\"\n"
        (
            cd "./$name"
            sed -i "" "s/25565/$port/g" server.json
            sed -i "" 's#"connection_mode": 1#"connection_mode": 3#g' server.json
            echo "Starting $name on port $port"
            "../bin/$name" &
        )
    else
        echo "Subdirectory ./$name does not exist, skipping $name."
    fi
done

# Remove the last newline
result=$(echo -e "$result" | sed '$d')

# Escape the newlines
result=${result//$'\n'/\\n}

sed -i "" "s/lobby = \"127.0.0.1:25566\"/$result/g" ./proxy/velocity.toml

cd ./proxy
$JAVA_HOME/bin/java -jar ./velocity.jar &

wait