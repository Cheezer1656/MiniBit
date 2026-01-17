#!/bin/bash

SECRET=$(LC_ALL=C tr -dc 'A-Za-z0-9' < /dev/urandom | head -c 12)
cat << EOF > .env.compose
MINIBIT_FORWARDING_SECRET=$SECRET
VELOCITY_FORWARDING_SECRET=$SECRET
EOF
