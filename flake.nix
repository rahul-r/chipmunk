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
            openssl.dev
            trunk
            sqlx-cli
            postgresql
            glibcLocales

            (writeShellScriptBin "start_database" ''
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

              # register_cleanup stop_db # ~ trapping HUP/EXIT
            '')
            (writeShellScriptBin "stop_database" "pg_ctl stop")
          ]);

          PGDATA = "./.tmp/db";
          PGHOST = "localhost";
          PGDATABASE = "chipmunk";
          PGUSER = "chipmunk";
          PGPASSWORD = "chipmunk";
          PGPORT = 5432;

          shellHook = ''
            pg_isready -t1 > /dev/null || start_database
            export DATABASE_URL="postgres://$PGUSER:$PGPASSWORD@$PGHOST:$PGPORT/$PGDATABASE"
            export CAR_DATA_DATABASE_URL="postgres://$PGUSER:$PGPASSWORD@$PGHOST:$PGPORT/$PGDATABASE"
            export TEST_DATABASE_URL="postgres://$PGUSER:$PGPASSWORD@$PGHOST:$PGPORT/$PGDATABASE"
          '';
      };
    });
  };
}
