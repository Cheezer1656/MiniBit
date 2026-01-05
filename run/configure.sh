#!/bin/bash

cd $(dirname "$0")

result=""
declare -i port=25566
for file in **/server.json; do
    name=$(dirname "$file")

    port+=1
    result+="    $name: localhost:$port\n"
    (
        cd "./$name"
`        config=$(<server.json)
        config=${config//25565/$port}
        config=${config//\"connection_mode\": 1/\"connection_mode\": 3}

        echo "$config" > server.json`
    )
done

# Remove the last newline
result=$(echo -e "$result" | sed '$d')

# Escape the newlines
result=${result//$'\n'/\\n}

sed -i "" "s/    lobby: localhost:25565/$result/g" ./proxy/config.yml
