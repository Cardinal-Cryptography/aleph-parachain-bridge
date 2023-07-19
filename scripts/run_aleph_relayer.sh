#!/bin/bash

echo "Starting AlephZero -> AlephParachain headers relay."
echo "---------------------------------------------------"

ALEPH_ZERO_PORT="${MILLAU_PORT:-9945}"
ALEPH_PARACHAIN_PORT="${ALEPH_PARACHAIN_PORT:?ALEPH_PARACHAIN_PORT must be set}"

./target/release/substrate-relay relay-headers \
	aleph-zero-to-aleph-parachain \
	--source-host localhost \
	--source-port $ALEPH_ZERO_PORT \
	--target-host localhost \
	--target-port $ALEPH_PARACHAIN_PORT \
	--target-signer //Bob \