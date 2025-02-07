// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use cid::Cid;
use fvm_shared::{
    address::Address, econ::TokenAmount, error::ExitCode, message::Message as FvmMessage,
    version::NetworkVersion,
};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use fendermint_vm_encoding::IsHumanReadable;

/// Queries over the IPLD blockstore or the state tree.
///
/// Maybe we can have some common queries over the known state of built-in actors,
/// and actors supporting IPC, or FEVM.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FvmQuery {
    /// Query something from the IPLD store.
    ///
    /// The response is the raw bytes from the store.
    Ipld(Cid),
    /// Query the state of an actor.
    ///
    /// The response is IPLD encoded `ActorState`.
    ActorState(Address),
    /// Immediately execute an FVM message, without adding it to the blockchain.
    ///
    /// The main motivation for this method is to facilitate `eth_call`.
    Call(Box<FvmMessage>),
    /// Estimate the gas required to execute a message.
    ///
    /// This is effectively a [`Call`], but it's included so that in the future
    /// it can do more sophisticated things with premiums, caps and over estimation.
    EstimateGas(Box<FvmMessage>),
    /// Retrieve the slowly changing state parameters that aren't part of the state tree.
    StateParams,
}

/// State of all actor implementations.
///
/// This is a copy of `fvm::state_tree::ActorState` so that this crate
/// doesn't need a dependency on `fvm` itself, only `fvm_shared`.
///
/// Note that it changes changes `Serialize_tuple` into `Serialize`
/// to preserve the field names; the intention is to display the results
/// as JSON, where tuple serialization wouldn't be as useful.
#[serde_as]
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ActorState {
    /// Link to code for the actor.
    #[serde_as(as = "IsHumanReadable")]
    pub code: Cid,
    /// Link to the state of the actor.
    #[serde_as(as = "IsHumanReadable")]
    pub state: Cid,
    /// Sequence of the actor.
    pub sequence: u64,
    /// Tokens available to the actor.
    #[serde_as(as = "IsHumanReadable")]
    pub balance: TokenAmount,
    /// The actor's "delegated" address, if assigned.
    ///
    /// This field is set on actor creation and never modified.
    #[serde_as(as = "Option<IsHumanReadable>")]
    pub delegated_address: Option<Address>,
}

/// Result of gas estimation.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct GasEstimate {
    /// Exit code, potentially signalling out-of-gas errors, or that the actor was not found.
    pub exit_code: ExitCode,
    /// Gas used during the probing.
    ///
    /// Potentially contains an over-estimate, but it should be within the account balance limit.
    pub gas_limit: u64,
}

/// Slowly changing state parameters outside the state tree.
#[serde_as]
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct StateParams {
    /// Base fee.
    ///
    /// Its evolution can depend on the size of blocks, contention, etc.
    #[serde_as(as = "IsHumanReadable")]
    pub base_fee: TokenAmount,
    /// Circulating supply.
    ///
    /// Its value depends on the amount moving in/out of the subnet.
    #[serde_as(as = "IsHumanReadable")]
    pub circ_supply: TokenAmount,
    /// Numeric chain ID for signing transactions.
    ///
    /// Its value is most likely fixed since genesis, but it might change during a fork.
    pub chain_id: u64,
    /// Current network version.
    pub network_version: NetworkVersion,
}

#[cfg(feature = "arb")]
mod arb {
    use fendermint_testing::arb::{ArbAddress, ArbCid, ArbTokenAmount};

    use crate::signed::SignedMessage;

    use super::{ActorState, FvmQuery};

    impl quickcheck::Arbitrary for FvmQuery {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            match u8::arbitrary(g) % 5 {
                0 => FvmQuery::Ipld(ArbCid::arbitrary(g).0),
                1 => FvmQuery::ActorState(ArbAddress::arbitrary(g).0),
                2 => FvmQuery::Call(Box::new(SignedMessage::arbitrary(g).into_message())),
                3 => FvmQuery::EstimateGas(Box::new(SignedMessage::arbitrary(g).into_message())),
                _ => FvmQuery::StateParams,
            }
        }
    }

    impl quickcheck::Arbitrary for ActorState {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            Self {
                code: ArbCid::arbitrary(g).0,
                state: ArbCid::arbitrary(g).0,
                sequence: u64::arbitrary(g),
                balance: ArbTokenAmount::arbitrary(g).0,
                delegated_address: Option::<ArbAddress>::arbitrary(g).map(|a| a.0),
            }
        }
    }
}
