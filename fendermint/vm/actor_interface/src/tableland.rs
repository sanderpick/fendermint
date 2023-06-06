use cid::multihash::Code;
use cid::Cid;
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_encoding::strict_bytes;
use fvm_ipld_encoding::tuple::*;
use fvm_ipld_encoding::CborStore;
use fvm_shared::METHOD_CONSTRUCTOR;

define_singleton!(TABLELAND {
    id: 17,
    code_id: 17
});

#[repr(u64)]
pub enum Method {
    Constructor = METHOD_CONSTRUCTOR,
    Query = 2,
}

#[derive(Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(transparent)]
pub struct QueryParams {
    pub statement: String,
}

#[derive(Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(transparent)]
pub struct QueryReturn {
    #[serde(with = "strict_bytes")]
    pub ret: Vec<u8>,
}

#[derive(Clone, Debug, Serialize_tuple, Deserialize_tuple)]
#[serde(transparent)]
pub struct DB {
    pub pages: Vec<Cid>,
}

impl DB {
    pub fn new(store: &impl Blockstore, data: &[u8], page_size: usize) -> Self {
        let len = data.len();
        let mut page_count = len / page_size;
        if len % page_size > 0 {
            page_count += 1;
        }

        let mut pages = vec![Cid::default(); page_count];
        for p in 0..page_count {
            let mut end = (p + 1) * page_size;
            if end > len {
                end = len;
            }
            let page = &data[p * page_size..end];
            pages[p] = store.put_cbor(&page, Code::Blake2b256).unwrap();
        }

        DB { pages }
    }
}

#[derive(Clone, Debug, Serialize_tuple, Deserialize_tuple)]
pub struct State {
    pub db: DB,
}
