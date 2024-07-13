#!/usr/bin/env bash

set -e

#USE_PODMAN=1

if [ -n "$USE_PODMAN" ]; then
  echo "Using podman"
  CONTAINER=podman
else
  echo "Using docker"
  CONTAINER=docker
fi

start_dev_env() {
  echo "Starting development environment"
  HOST_UID=$(id -u)
  HOST_GID=$(id -g)
  export HOST_UID
  export HOST_GID

  mkdir -p chipmunk_container/postgres
  mkdir -p chipmunk_container/pgadmin
  mkdir -p chipmunk_container/grafana
  touch chipmunk_container/.bash_history

  # sudo chown -R 472:0 chipmunk_container/grafana
  # sudo chown -R 5050:5050 chipmunk_container/pgadmin

  set +e
  # https://docs.docker.com/engine/reference/commandline/compose_run/
  if [ -n "$USE_PODMAN" ]; then
    podman compose -f docker-compose-dev.yml run \
                      --rm \
                      --remove-orphans \
                      --service-ports \
                      --name chipmunk-dev \
                      chipmunk-dev
  else
    docker compose -f docker-compose-dev.yml run \
                      --rm \
                      --service-ports \
                      --name chipmunk-dev \
                      chipmunk-dev
  fi
  $CONTAINER compose -f docker-compose-dev.yml down
}

build_dev_env() {
  echo "Rebuilding development environment"
  HOST_UID=$(id -u)
  HOST_GID=$(id -g)
  export HOST_UID
  export HOST_GID

  $CONTAINER compose -f docker-compose-dev.yml down
  $CONTAINER compose -f docker-compose-dev.yml build --no-cache
}

redo_migration() {
  echo "Redoing migration"
  pushd chipmunk
  NUM_UP_SCRIPTS=$(ls migrations/*up.sql | wc -l)
  NUM_DOWN_SCRIPTS=$(ls migrations/*down.sql | wc -l)

  for i in `eval echo {1..$NUM_DOWN_SCRIPTS}`; do
    sqlx migrate revert
  done

  sqlx migrate run
  popd
}

case $1 in
  "start")
    start_dev_env
    ;;
  "build")
    build_dev_env
    ;;
  "redo-migration")
    redo_migration
    ;;
  *)
    echo "Usage: $0 {start|redo-migration}"
    exit 1
    ;;
esac
