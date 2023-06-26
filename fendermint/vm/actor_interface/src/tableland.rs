define_singleton!(TABLELAND {
    id: 17,
    code_id: 17
});

pub use fil_actor_tableland_interface::{
    ConstructorParams, ExecuteParams, ExecuteReturn, Method, QueryParams, QueryReturn, State, DB,
    SQLITE_PAGE_SIZE,
};
