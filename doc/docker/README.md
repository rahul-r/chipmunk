# Development Environment Using Docker Containers

## Setup development environment using docker

Copy these files into the project root directory
[Dockerfile.dev](./Dockerfile.dev)
[docker-compose-dev.yml](./docker-compose-dev.yml)
[dev-environment.sh](./dev-environment.sh)

Create a `.env` file with the secrets used in the docker-compose file. See `env.example` file in the project root for the required environment variables.

Use the [dev-environment.sh](./dev-environment.sh) shell script for easy execution of development tasks

Build the development containers
```sh
./dev-environment.sh build 
```

Start development containers and drop to rust container shell
```sh
./dev-environment.sh start
```

Redo database migrations

```sh 
./dev-environment.sh redo-migration
```

## Development set on macOS

Development setup on mac uses podman instead of docker to run the containers.

Update `Dockerfile.dev` as follows
```diff
-    curl -LsSf https://get.nexte.st/latest/linux | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin
+    curl -LsSf https://get.nexte.st/latest/linux-arm | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin

...

- RUN groupadd -g "${GID}" "${USERNAME}"
+# RUN groupadd -g "${GID}" "${USERNAME}"
```

Replace `docker-compose-dev.yml` with this
```yml
# docker-compose-dev.yml
services:
  db:
    image: postgres:16
    container_name: db
    hostname: db
    restart: unless-stopped
    ports:
      - "5432:5432"
    security_opt:
      - "label=disable"
    # volumes:
    #   - ./chipmunk_container/postgres:/var/lib/postgresql/data:rw
    environment:
      POSTGRES_USER: chipmunk
      POSTGRES_PASSWORD: chipmunk
      POSTGRES_DB: chipmunk
```

Create a new script to start the new mac development setup
```sh
start-mac-dev.sh
#!/usr/bin/env bash

podman machine start

export DOCKER_HOST='unix:///Users/rahul/.local/share/containers/podman/machine/qemu/podman.sock'

# ./dev-environment.sh start
podman compose -f docker-compose-dev.yml run \
	--rm \
	--remove-orphans \
	--service-ports \
	--name db \
	db
```
