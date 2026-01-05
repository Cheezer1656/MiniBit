#!/bin/bash

cd $(dirname "$0")

./configure.sh

../bin/minibit --auto . &

GATE_VELOCITY_SECRET=$(<proxy/forwarding.secret) ../bin/gate -c proxy/config.yml &

wait

