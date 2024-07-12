{
  inputs = {
    # nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default";
  };

  outputs = {
    systems,
    nixpkgs,
    ...
  } @ inputs: let
    eachSystem = f:
      nixpkgs.lib.genAttrs (import systems) (
        system:
          f nixpkgs.legacyPackages.${system}
      );
  in {
    packages =
      eachSystem (pkgs: {
      });

    devShells = eachSystem (pkgs: {
      default = pkgs.mkShell {
        buildInputs = with pkgs; ([
            rustup
            trunk
            sqlx-cli
            cargo-nextest

            openssl.dev
            postgresql
            glibcLocales

            (writeShellScriptBin "chipmunk-start-postgres" ''
              set -e

              start_db() {
                pg_ctl -w -l $PGDATA/log start -o "-k /tmp"
              }

              stop_db() {
                pg_ctl stop
              }

              if [ ! -d $PGDATA ]; then
                initdb --auth=password --pwfile=<(echo $PGPASSWORD)
                start_db
                createdb -U $(whoami)
                createuser -U $(whoami) --superuser
                psql -U $(whoami) -c "ALTER USER $PGUSER WITH PASSWORD '$PGPASSWORD';"
              else
                start_db
              fi
            '')
            (writeShellScriptBin "chipmunk-stop-db" "pg_ctl stop")
            (writeShellScriptBin "chipmunk-build-offline_db" ''
              cargo sqlx prepare --workspace -- --all-targets --all-features
            '')
            (writeShellScriptBin "chipmunk-redo-migration" ''
              set -e
              pushd chipmunk
              NUM_UP_SCRIPTS=$(ls migrations/*up.sql | wc -l)
              NUM_DOWN_SCRIPTS=$(ls migrations/*down.sql | wc -l)

              for i in `eval echo {1..$NUM_DOWN_SCRIPTS}`; do
                sqlx migrate revert
              done

              sqlx migrate run
              popd
            '')
          ]);

          PGDATA = "./.tmp/db";
          PGHOST = "localhost";
          PGDATABASE = "chipmunk";
          PGUSER = "chipmunk";
          PGPASSWORD = "chipmunk";
          PGPORT = 5432;

          shellHook = ''
            pg_isready -t1 > /dev/null || chipmunk-stop-postgres
            export DATABASE_URL="postgres://$PGUSER:$PGPASSWORD@$PGHOST:$PGPORT/$PGDATABASE"
            export CAR_DATA_DATABASE_URL="postgres://$PGUSER:$PGPASSWORD@$PGHOST:$PGPORT/$PGDATABASE"
            export TEST_DATABASE_URL="postgres://$PGUSER:$PGPASSWORD@$PGHOST:$PGPORT/$PGDATABASE"
          '';
      };
    });
  };
}
