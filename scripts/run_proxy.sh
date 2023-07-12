#!/bin/sh
set -eu

fendermint proxy start --secret-key test-network/keys/alice.sk --sequence 0 --chain-name test --gas-limit 100000000000
