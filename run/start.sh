#!/bin/sh

cd $(dirname "$0")

cd ./proxy
$JAVA_HOME/bin/java -jar ./velocity.jar &

cd ..

for EXEC_FILE in ./bin/*; do
    FILENAME=$(basename "$EXEC_FILE")

    # Define the subdirectory path
    SUBDIR="./$FILENAME"

    # Check if the subdirectory exists
    if [ -d "$SUBDIR" ]; then
        # Run the executable inside its respective subdirectory
        (cd "$SUBDIR" && "../bin/$FILENAME")
    else
        echo "Subdirectory $SUBDIR does not exist, skipping $FILENAME."
    fi
done

wait