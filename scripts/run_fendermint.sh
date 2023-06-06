#!/bin/sh

rm -rf ~/.fendermint/data/rocksdb
cargo install --path fendermint/app
fendermint --log-level debug run
