#!/usr/bin/env bash
set -euo pipefail

scale=$1
cd "$(dirname "${BASH_SOURCE[0]}")/.."

cat <<'EOT'
version: '2'

services:
    coordinator:
        container_name: coordinator
        build:
            context: .
            dockerfile: ./docker/Dockerfile.coord
            args:
                FLAVOR:
                BIND_ADDR: 10.1.0.2
                BIND_PORT: 4001
                DITTO_APP_ID: "${DITTO_APP_ID}"
                DITTO_PG_TOKEN: "${DITTO_PG_TOKEN}"
                DITTO_LICENSE: "${DITTO_LICENSE}"

        expose: ["4000-4099"]
#       ports:
#          - "4000-4099:4000-4099"
        networks:
          mesh:
            ipv4_address: 10.1.0.2
EOT

for (( i = 0; i<scale; i++)); do
    block_sz=10
    beginport=$((5100 + ( i * block_sz) ))
    endport=$((beginport + block_sz - 1))
    cat <<EOT
    peer$i:
        container_name: peer$i
        build:
            context: .
            dockerfile: ./docker/Dockerfile.peer
            args:
                DEVICE_NAME: "peer$i"
                FLAVOR:
                BIND_PORT: $beginport
                COORD_ADDR: 10.1.0.2
                COORD_PORT: 4001
EOT

    cat <<'EOT'
                DITTO_APP_ID: "${DITTO_APP_ID}"
                DITTO_PG_TOKEN: "${DITTO_PG_TOKEN}"
                DITTO_LICENSE: "${DITTO_LICENSE}"
EOT
    cat <<EOT
        expose: ["$beginport-$endport"]
        networks:
          - mesh
EOT
done

cat <<EOT
networks:
    mesh:
        driver: bridge
        ipam:
            config:
                - subnet: 10.1.0.0/16
                  gateway: 10.1.0.1
                  # keep static IPs from clashing w/ assigned
                  ip_range: 10.1.1.0/24
EOT

