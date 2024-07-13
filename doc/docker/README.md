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
