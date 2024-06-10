FROM rust:bookworm AS builder

# Build backend
WORKDIR /chipmunk/
COPY . .
RUN cargo build --release

# Build frontend
# Install and run tailwindcss
RUN wget https://github.com/tailwindlabs/tailwindcss/releases/download/v3.4.4/tailwindcss-linux-x64 -O tailwindcss && \
    chmod +x tailwindcss && \
    ./tailwindcss -i ./style/tailwind.css -o ./public/styles.css

# Install trunk
# RUN cargo install --locked trunk # This might take a long time. Install a prebuilt package as a workaround
ARG TRUNK_VERSION=v0.17.1
RUN wget -qO- https://github.com/thedodd/trunk/releases/download/${TRUNK_VERSION}/trunk-x86_64-unknown-linux-gnu.tar.gz | tar -xzf- -C /usr/bin/
WORKDIR /chipmunk/ui/frontend
RUN trunk build

# Create release version using grafana as base image
FROM grafana/grafana:10.2.1-ubuntu

ENV GF_ANALYTICS_REPORTING_ENABLED=FALSE \
    GF_AUTH_ANONYMOUS_ENABLED=false \
    GF_AUTH_BASIC_ENABLED=false \
    GF_PATHS_PLUGINS="/var/lib/grafana-plugins" \
    GF_PLUGINS_ALLOW_LOADING_UNSIGNED_PLUGINS=natel-discrete-panel,pr0ps-trackmap-panel,panodata-map-panel,natel-plotly-panel \
    GF_SECURITY_ADMIN_PASSWORD=admin \
    GF_SECURITY_ADMIN_USER=admin \
    GF_SECURITY_ALLOW_EMBEDDING=true \
    GF_SECURITY_DISABLE_GRAVATAR=true \
    GF_USERS_ALLOW_SIGN_UP=false \
    GF_ANALYTICS_REPORTING_ENABLED=FALSE \
    DATABASE_PORT=5432

USER root

RUN sed -i '15 i mkdir -p "${GF_PATHS_DATA}"\nchown -R grafana "${GF_PATHS_DATA}"\n' /run.sh

RUN mkdir -p "$GF_PATHS_PLUGINS" && \
    chown -R grafana "$GF_PATHS_PLUGINS"

RUN apt-get update && apt-get install -y libssl-dev && rm -rf /var/lib/apt/lists/*

COPY --from=builder /chipmunk/target/release/chipmunk /chipmunk/chipmunk
COPY --from=builder /chipmunk/ui/frontend/dist /chipmunk/dist

USER grafana

RUN grafana-cli --pluginsDir "${GF_PATHS_PLUGINS}" plugins install pr0ps-trackmap-panel 2.1.4 && \
    grafana-cli --pluginsDir "${GF_PATHS_PLUGINS}" plugins install natel-plotly-panel 0.0.7 && \
    grafana-cli --pluginsDir "${GF_PATHS_PLUGINS}" plugins install golioth-websocket-datasource 1.0.2 && \
    grafana-cli --pluginsDir "${GF_PATHS_PLUGINS}" --pluginUrl https://github.com/panodata/panodata-map-panel/releases/download/0.16.0/panodata-map-panel-0.16.0.zip plugins install grafana-worldmap-panel-ng

COPY grafana/logo.svg /usr/share/grafana/public/img/grafana_icon.svg
COPY grafana/favicon.png /usr/share/grafana/public/img/fav32.png
COPY grafana/apple-touch-icon.png /usr/share/grafana/public/img/apple-touch-icon.png

COPY grafana/datasource.yml /etc/grafana/provisioning/datasources/
COPY grafana/dashboards.yml /etc/grafana/provisioning/dashboards/
COPY grafana/dashboards/internal/*.json /dashboards_internal/
COPY grafana/dashboards/*.json /dashboards/

EXPOSE 3000

# Create script to start chipmunk and grafana
USER root
RUN echo "#!/bin/bash\n"\
    "export GF_DASHBOARDS_DEFAULT_HOME_DASHBOARD_PATH=/dashboards/overview.json\n"\
    "export GF_PANELS_DISABLE_SANITIZE_HTML=true\n"\
    "export HTTP_PORT=${HTTP_PORT:=3072}\n"\
    "export CHIPMUNK_HOST=localhost\n"\
    "export CHIPMUNK_PORT=${HTTP_PORT}\n"\
    "/run.sh& /chipmunk/chipmunk tasks"\
    > /chipmunk/start.sh &&\
    chmod +x /chipmunk/start.sh

USER grafana

ENTRYPOINT [ "/chipmunk/start.sh" ]
