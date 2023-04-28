#!/bin/bash

# Script for local runs of Millau<>AlephParachain bridge with init + complex relay
#
# Need running Millau node and AlephParachain + Rococo

MILLAU_PORT="${MILLAU_PORT:-9945}"
ROCOCO_PORT="${ROCOCO_PORT:?ROCOCO_PORT variable must be set}"
ALEPH_PARACHAIN_PORT="${ALEPH_PARACHAIN_PORT:?ALEPH_PARACHAIN_PORT must be set}"

echo "Initializing Millau <> AlephParachain bridge"

./target/debug/substrate-relay init-bridge \
    millau-to-aleph-parachain \
	--source-host localhost \
	--source-port $MILLAU_PORT \
	--target-host localhost \
	--target-port $ALEPH_PARACHAIN_PORT \
	--target-signer //Alice

./target/debug/substrate-relay init-bridge \
	rococo-to-millau \
	--source-host localhost \
	--source-port $ROCOCO_PORT \
	--target-host localhost \
	--target-port $MILLAU_PORT \
	--target-signer //Sudo

echo "Starting Millau <> AlephParachain complex relay"
./target/debug/substrate-relay relay-headers-and-messages \
	aleph-parachain-millau \
	--aleph-parachain-host localhost \
	--aleph-parachain-port $ALEPH_PARACHAIN_PORT \
	--aleph-parachain-signer //Bob \
	--millau-host localhost \
	--millau-port $MILLAU_PORT \
	--millau-signer //Bob \
	--rococo-host localhost \
	--rococo-port $ROCOCO_PORT 