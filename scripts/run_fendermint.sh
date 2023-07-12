#!/bin/sh
set -eu

rm -rf ~/.fendermint/data/rocksdb
fendermint --log-level debug run
