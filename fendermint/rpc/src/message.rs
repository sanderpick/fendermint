// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use std::path::Path;

use anyhow::Context;
use base64::Engine;
use bytes::Bytes;
use fendermint_vm_actor_interface::{eam, evm};
use fendermint_vm_message::{chain::ChainMessage, signed::SignedMessage};
use fvm_ipld_encoding::{BytesSer, RawBytes};
use fvm_shared::{
    address::Address, chainid::ChainID, econ::TokenAmount, message::Message, MethodNum, METHOD_SEND,
};
use libsecp256k1::{PublicKey, SecretKey};

use crate::B64_ENGINE;

/// Factory methods for signed transaction payload construction.
pub struct MessageFactory {
    sk: SecretKey,
    addr: Address,
    sequence: u64,
    chain_id: ChainID,
}

impl MessageFactory {
    pub fn new(sk: SecretKey, sequence: u64, chain_id: ChainID) -> anyhow::Result<Self> {
        let pk = PublicKey::from_secret_key(&sk);
        let addr = Address::new_secp256k1(&pk.serialize())?;
        Ok(Self {
            sk,
            addr,
            sequence,
            chain_id,
        })
    }

    /// Convenience method to read the secret key from a file, expected to be in Base64 format.
    pub fn read_secret_key(sk: &Path) -> anyhow::Result<SecretKey> {
        let b64 = std::fs::read_to_string(sk).context("failed to read secret key")?;
        let bz: Vec<u8> = B64_ENGINE
            .decode(b64)
            .context("failed to parse base64 string")?;
        let sk = SecretKey::parse_slice(&bz)?;
        Ok(sk)
    }

    /// Convenience method to serialize a [`ChainMessage`] for inclusion in a Tendermint transaction.
    pub fn serialize(message: &ChainMessage) -> anyhow::Result<Vec<u8>> {
        Ok(fvm_ipld_encoding::to_vec(message)?)
    }

    /// Actor address.
    pub fn address(&self) -> &Address {
        &self.addr
    }

    /// Set the sequence to an arbitrary value.
    pub fn set_sequence(&mut self, sequence: u64) {
        self.sequence = sequence;
    }

    /// Transfer tokens to another account.
    pub fn transfer(
        &mut self,
        to: Address,
        value: TokenAmount,
        gas_params: GasParams,
    ) -> anyhow::Result<ChainMessage> {
        self.transaction(to, METHOD_SEND, Default::default(), value, gas_params)
    }

    /// Send a message to an actor.
    pub fn transaction(
        &mut self,
        to: Address,
        method_num: MethodNum,
        params: RawBytes,
        value: TokenAmount,
        gas_params: GasParams,
    ) -> anyhow::Result<ChainMessage> {
        let message = fvm_shared::message::Message {
            version: Default::default(), // TODO: What does this do?
            from: self.addr,
            to,
            sequence: self.sequence,
            value,
            method_num,
            params,
            gas_limit: gas_params.gas_limit,
            gas_fee_cap: gas_params.gas_fee_cap,
            gas_premium: gas_params.gas_premium,
        };
        self.sequence += 1;
        let signed = SignedMessage::new_secp256k1(message, &self.sk, &self.chain_id)?;
        let chain = ChainMessage::Signed(Box::new(signed));
        Ok(chain)
    }

    /// Deploy a FEVM contract.
    pub fn fevm_create(
        &mut self,
        contract: Bytes,
        constructor_args: Bytes,
        value: TokenAmount,
        gas_params: GasParams,
    ) -> anyhow::Result<ChainMessage> {
        let initcode = [contract.to_vec(), constructor_args.to_vec()].concat();
        let initcode = RawBytes::serialize(BytesSer(&initcode))?;
        let message = self.transaction(
            eam::EAM_ACTOR_ADDR,
            eam::Method::CreateExternal as u64,
            initcode,
            value,
            gas_params,
        )?;
        Ok(message)
    }

    /// Invoke a method on a FEVM contract.
    pub fn fevm_invoke(
        &mut self,
        contract: Address,
        calldata: Bytes,
        value: TokenAmount,
        gas_params: GasParams,
    ) -> anyhow::Result<ChainMessage> {
        let calldata = RawBytes::serialize(BytesSer(&calldata))?;
        let message = self.transaction(
            contract,
            evm::Method::InvokeContract as u64,
            calldata,
            value,
            gas_params,
        )?;
        Ok(message)
    }

    /// Create a message for a read-only operation.
    pub fn fevm_call(
        &mut self,
        contract: Address,
        calldata: Bytes,
        value: TokenAmount,
        gas_params: GasParams,
    ) -> anyhow::Result<Message> {
        let msg = self.fevm_invoke(contract, calldata, value, gas_params)?;

        let msg = if let ChainMessage::Signed(signed) = msg {
            signed.into_message()
        } else {
            panic!("unexpected message type: {msg:?}");
        };

        // Roll back the sequence, we don't really want to invoke anything.
        self.set_sequence(msg.sequence);

        Ok(msg)
    }
}

#[derive(Clone, Debug)]
pub struct GasParams {
    /// Maximum amount of gas that can be charged.
    pub gas_limit: u64,
    /// Price of gas.
    ///
    /// Any discrepancy between this and the base fee is paid for
    /// by the validator who puts the transaction into the block.
    pub gas_fee_cap: TokenAmount,
    /// Gas premium.
    pub gas_premium: TokenAmount,
}
