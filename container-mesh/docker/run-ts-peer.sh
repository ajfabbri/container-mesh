#!/usr/bin/env bash
set -euo pipefail
source .env
source .secret.env

echo "Running docker/run-ts-peer.sh from $(pwd)"
echo "Current directory contents:\
$(ls -l)"

for var in DITTO_APP_ID DITTO_PG_TOKEN DITTO_LICENSE; do
    if [[ ! -v $var ]]; then
        echo "Error: $var is not set"
        exit 1
    fi
done

set -x
npm run start -- $@
set +x

echo "Finished docker/run-ts-peer.sh"
sleep 2
