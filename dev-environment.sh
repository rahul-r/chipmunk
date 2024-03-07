#!/usr/bin/env bash

set -e

start_dev_env() {
  echo "Starting development environment"
  HOST_UID=$(id -u)
  HOST_GID=$(id -g)
  export HOST_UID
  export HOST_GID

  mkdir -p chipmunk_docker/postgres
  mkdir -p chipmunk_docker/pgadmin
  mkdir -p chipmunk_docker/grafana

  sudo chown -R 472:0 chipmunk_docker/grafana
  sudo chown -R 5050:5050 chipmunk_docker/pgadmin

  set +e
  # https://docs.docker.com/engine/reference/commandline/compose_run/
  podman compose -f docker-compose-dev.yml run \
                    --rm \
                    --service-ports \
                    --name chipmunk-dev \
                    chipmunk-dev

  podman-compose -f docker-compose-dev.yml down
}

build_dev_env() {
  echo "Rebuilding development environment"
  HOST_UID=$(id -u)
  HOST_GID=$(id -g)
  export HOST_UID
  export HOST_GID

  podman-compose -f docker-compose-dev.yml down
  podman-compose -f docker-compose-dev.yml build --no-cache
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
