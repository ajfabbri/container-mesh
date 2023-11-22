#!/usr/bin/env bash
set -euo pipefail

scale=$1
cd "$(dirname "${BASH_SOURCE[0]}")/.."
source .env

cat <<'EOT'
version: '2'

services:
    coordinator:
        container_name: coordinator
        env_file:
          - .env
          - .secret.env
        build:
            context: .
            dockerfile: ./docker/Dockerfile.coord
            args:
                FLAVOR:
EOT
    cat <<'EOT'
                ARCH: ${ARCH}
EOT
    cat <<EOT
                BIND_ADDR: 10.1.0.2
                BIND_PORT: 4001

        expose: ["4000-4099"]
        networks:
          mesh:
            ipv4_address: 10.1.0.2
EOT

for (( i = 1; i<=scale; i++)); do
    block_sz=10
    beginport=$((5100 + ( i * block_sz) ))
    endport=$((beginport + block_sz - 1))
    cat <<EOT

    peer$i:
        container_name: peer$i
        env_file:
          - .env
          - .secret.env
        build:
            context: .
            dockerfile: ./docker/Dockerfile.peer
            args:
                FLAVOR:
EOT
    cat <<'EOT'
                ARCH: ${ARCH}
EOT
    cat <<EOT
                COORD_ADDR: 10.1.0.2
                COORD_PORT: 4001
                BIND_PORT: $beginport
                DEVICE_NAME: peer$i
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

