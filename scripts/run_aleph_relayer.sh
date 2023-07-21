#!/bin/bash

ALEPH_ZERO_PORT="${ALEPH_ZERO_PORT:-9945}"
ALEPH_PARACHAIN_PORT="${ALEPH_PARACHAIN_PORT:?ALEPH_PARACHAIN_PORT must be set}"

echo "Initializing Aleph Zero -> AlephParachain bridge"
echo "------------------------------------------------"

./target/release/substrate-relay init-bridge \
    aleph-zero-to-aleph-parachain \
	--source-host localhost \
	--source-port $ALEPH_ZERO_PORT \
	--target-host localhost \
	--target-port $ALEPH_PARACHAIN_PORT \
	--target-signer //Alice

echo "Starting AlephZero -> AlephParachain headers relay."
echo "---------------------------------------------------"

./target/release/substrate-relay relay-headers \
	aleph-zero-to-aleph-parachain \
	--source-host localhost \
	--source-port $ALEPH_ZERO_PORT \
	--target-host localhost \
	--target-port $ALEPH_PARACHAIN_PORT \
	--target-signer //Bob \