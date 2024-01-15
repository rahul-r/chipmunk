#!/usr/bin/env bash

set -e

HOST_UID=$(id -u)
HOST_GID=$(id -g)
export HOST_UID
export HOST_GID

# Comment this line to use real tesla servers
export SIMULATE=yes  # yes or no

mkdir -p chipmunk_docker/postgres
mkdir -p chipmunk_docker/pgadmin
mkdir -p chipmunk_docker/grafana

sudo chown -R 472:0 chipmunk_docker/grafana
sudo chown -R 5050:5050 chipmunk_docker/pgadmin

# https://docs.docker.com/engine/reference/commandline/compose_run/
docker-compose -f docker-compose-dev.yml run \
                  --rm \
                  --build \
                  --remove-orphans \
                  --service-ports \
                  --name chipmunk-dev \
                  chipmunk-dev

docker-compose -f docker-compose-dev.yml down