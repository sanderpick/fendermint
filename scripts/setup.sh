#!/bin/sh

cargo install --path fendermint/app

# Create a new Genesis file
rm -rf test-network
mkdir test-network
fendermint genesis --genesis-file test-network/genesis.json new --chain-name test --base-fee 1000 --timestamp 1680101412

# Create some keys
mkdir test-network/keys
for NAME in alice bob charlie dave; do
  fendermint key gen --out-dir test-network/keys --name $NAME;
done

# Add accounts to the Genesis file
## A stand-alone account
fendermint genesis --genesis-file test-network/genesis.json add-account --public-key test-network/keys/alice.pk --balance 1000000000000000000
## A multi-sig account
fendermint genesis --genesis-file test-network/genesis.json add-multisig --public-key test-network/keys/bob.pk --public-key test-network/keys/charlie.pk --public-key test-network/keys/dave.pk --threshold 2 --vesting-start 0 --vesting-duration 1000000 --balance 3000000000000000000

# Add validators to the Genesis file
fendermint genesis --genesis-file test-network/genesis.json add-validator --public-key test-network/keys/bob.pk --power 1

# Configure Tendermint
rm -rf ~/.cometbft
cometbft init

## Convert the Genesis file
mv ~/.cometbft/config/genesis.json ~/.cometbft/config/genesis.json.orig
fendermint genesis --genesis-file test-network/genesis.json into-tendermint --out ~/.cometbft/config/genesis.json
## Convert the private key
mv ~/.cometbft/config/priv_validator_key.json ~/.cometbft/config/priv_validator_key.json.orig
fendermint key into-tendermint --secret-key test-network/keys/bob.sk --out ~/.cometbft/config/priv_validator_key.json

## Setup data directory and copy default app config
rm -rf ~/.fendermint
mkdir -p ~/.fendermint/data
cp -r ./fendermint/app/config ~/.fendermint/config

# Build actors
make actor-bundle
cp ../builtin-actors/output/bundle.car ~/.fendermint/bundle.car
