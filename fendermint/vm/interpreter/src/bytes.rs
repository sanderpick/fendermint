// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use fendermint_vm_genesis::Genesis;
use fendermint_vm_message::chain::ChainMessage;

use crate::{
    chain::{ChainMessageApplyRet, ChainMessageCheckRet},
    fvm::{FvmQuery, FvmQueryRet},
    CheckInterpreter, ExecInterpreter, GenesisInterpreter, QueryInterpreter,
};

pub type BytesMessageApplyRet = Result<ChainMessageApplyRet, fvm_ipld_encoding::Error>;
pub type BytesMessageCheckRet = Result<ChainMessageCheckRet, fvm_ipld_encoding::Error>;
pub type BytesMessageQueryRet = Result<FvmQueryRet, fvm_ipld_encoding::Error>;

/// Close to what the ABCI sends: (Path, Bytes).
pub type BytesMessageQuery = (String, Vec<u8>);

/// Interpreter working on raw bytes.
#[derive(Clone)]
pub struct BytesMessageInterpreter<I> {
    inner: I,
}

impl<I> BytesMessageInterpreter<I> {
    pub fn new(inner: I) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<I> ExecInterpreter for BytesMessageInterpreter<I>
where
    I: ExecInterpreter<Message = ChainMessage, DeliverOutput = ChainMessageApplyRet>,
{
    type State = I::State;
    type Message = Vec<u8>;
    type BeginOutput = I::BeginOutput;
    type DeliverOutput = BytesMessageApplyRet;
    type EndOutput = I::EndOutput;

    async fn deliver(
        &self,
        state: Self::State,
        msg: Self::Message,
    ) -> anyhow::Result<(Self::State, Self::DeliverOutput)> {
        match fvm_ipld_encoding::from_slice::<ChainMessage>(&msg) {
            Err(e) =>
            // TODO: Punish the validator for including rubbish.
            // There is always the possibility that our codebase is incompatible,
            // but then we'll have a consensus failure later when we don't agree on the ledger.
            {
                Ok((state, Err(e)))
            }
            Ok(msg) => {
                let (state, ret) = self.inner.deliver(state, msg).await?;
                Ok((state, Ok(ret)))
            }
        }
    }

    async fn begin(&self, state: Self::State) -> anyhow::Result<(Self::State, Self::BeginOutput)> {
        self.inner.begin(state).await
    }

    async fn end(&self, state: Self::State) -> anyhow::Result<(Self::State, Self::EndOutput)> {
        self.inner.end(state).await
    }
}

#[async_trait]
impl<I> CheckInterpreter for BytesMessageInterpreter<I>
where
    I: CheckInterpreter<Message = ChainMessage, Output = ChainMessageCheckRet>,
{
    type State = I::State;
    type Message = Vec<u8>;
    type Output = BytesMessageCheckRet;

    async fn check(
        &self,
        state: Self::State,
        msg: Self::Message,
        is_recheck: bool,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        match fvm_ipld_encoding::from_slice::<ChainMessage>(&msg) {
            Err(e) =>
            // The user sent us an invalid message, all we can do is discard it and block the source.
            {
                Ok((state, Err(e)))
            }
            Ok(msg) => {
                let (state, ret) = self.inner.check(state, msg, is_recheck).await?;
                Ok((state, Ok(ret)))
            }
        }
    }
}

#[async_trait]
impl<I> QueryInterpreter for BytesMessageInterpreter<I>
where
    I: QueryInterpreter<Query = FvmQuery, Output = FvmQueryRet>,
{
    type State = I::State;
    type Query = BytesMessageQuery;
    type Output = BytesMessageQueryRet;

    async fn query(
        &self,
        state: Self::State,
        qry: Self::Query,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        let (path, bz) = qry;
        let qry = if path.as_str() == "/store" {
            // According to the docstrings, the application MUST interpret `/store` as a query on the underlying KV store.
            match fvm_ipld_encoding::from_slice::<Cid>(&bz) {
                Err(e) => return Ok((state, Err(e))),
                Ok(cid) => FvmQuery::Ipld(cid),
            }
        } else {
            // Otherwise ignore the path for now. The docs also say that the query bytes can be used in lieu of the path,
            // so it's okay to have two ways to send IPLD queries: either by using the `/store` path and sending a CID,
            // or by sending the appropriate `FvmQuery`.
            match fvm_ipld_encoding::from_slice::<FvmQuery>(&bz) {
                Err(e) => return Ok((state, Err(e))),
                Ok(qry) => qry,
            }
        };

        let (state, ret) = self.inner.query(state, qry).await?;

        Ok((state, Ok(ret)))
    }
}

#[async_trait]
impl<I> GenesisInterpreter for BytesMessageInterpreter<I>
where
    I: GenesisInterpreter<Genesis = Genesis>,
{
    type State = I::State;
    type Genesis = Vec<u8>;
    type Output = I::Output;

    async fn init(
        &self,
        state: Self::State,
        genesis: Self::Genesis,
    ) -> anyhow::Result<(Self::State, Self::Output)> {
        // TODO (IPC-44): Handle the serialized application state as well as `Genesis`.
        let genesis: Genesis = parse_genesis(&genesis)?;
        self.inner.init(state, genesis).await
    }
}

/// Parse the initial genesis either as JSON or CBOR.
fn parse_genesis(bytes: &[u8]) -> anyhow::Result<Genesis> {
    try_parse_genesis_json(bytes).or_else(|e1| {
        try_parse_genesis_cbor(bytes)
            .map_err(|e2| anyhow!("failed to deserialize genesis as JSON or CBOR: {e1}; {e2}"))
    })
}

fn try_parse_genesis_json(bytes: &[u8]) -> anyhow::Result<Genesis> {
    let json = String::from_utf8(bytes.to_vec())?;
    let genesis = serde_json::from_str(&json)?;
    Ok(genesis)
}

fn try_parse_genesis_cbor(bytes: &[u8]) -> anyhow::Result<Genesis> {
    let genesis = fvm_ipld_encoding::from_slice(bytes)?;
    Ok(genesis)
}
