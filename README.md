# Chipmunk

## Install rust

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Customize the installation as desired or just continue with the default options.

To get started you may need to restart your current shell. This would reload your PATH environment variable to include Cargo's bin directory ($HOME/.cargo/bin).
To configure your current shell, run: `source "$HOME/.cargo/env"`

## Install dependencies

```shell
# Debian/Ubuntu
sudo apt install libssl-dev

# OpenSUSE
sudo zypper install libopenssl-3-devel

# Fedora
sudo dnf install openssl-devel
```

## Create .env
Copy `env.example` to `.env` and fill in the values.

## Build the project

```shell
cargo build
```

## Setup Development Environment

Run this command to start the development containers and drop into rust build container.

```shell
./dev-environment.sh start
```

The application can be built with `cargo build` in the rust development container.


## Building docker image

```shell
docker-compose build
```

## Build and run the container

```shell
docker-compose up --build --force-recreate --remove-orphans
```

## Using the container

- Create mount points
```shell
mkdir -p chipmunk_docker/postgres
mkdir -p chipmunk_docker/grafana

sudo chown -R 472:0 chipmunk_docker/grafana
sudo chown -R 5050:5050 chipmunk_docker/pgadmin
```

- Start the container
```shell
docker-compose up
```

## Building offline without database access

The sqlx crate requires access to a database to compile successfully. Follow these steps to build the project offline, without active database connection.

sqlx might fail to generate the files needed to build offline if the versions of sqlx and sqlx-cli are not the same. To ensure that the versions are the same, run the following command to install sqlx-cli:

```shell
cargo install sqlx-cli
```

Then run the following command to generate the files needed to build offline:

```shell
cargo sqlx prepare --workspace -- --all-targets --all-features
```

### Force building in offline mode

The presence of a `DATABASE_URL` environment variable will take precedence over the presence of `.sqlx`, meaning SQLx will default to building against a database if it can. To make sure an accidentally-present `DATABASE_URL` environment variable or `.env` file does not result in cargo build (trying to) access the database, set the `SQLX_OFFLINE` environment variable to true.

To make this the default, add it to `.env` file. cargo sqlx prepare will still do the right thing and connect to the database.

## Testing
`cargo test` will run tests parallely in different threads. Some of the tests in this project uses shared resources (database, http server, etc.) and will fail `cargo test`. One option is to run the tests sequentially which will take a long time to run the tests.

Alternatively, we can use test frameworks like cargo-nextest to run the tests parallely without interferring with each other.

```shell
cargo install cargo-nextest
cargo nextest run
```