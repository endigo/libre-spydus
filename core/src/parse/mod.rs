//! OPAC HTML → typed records. Each module is a faithful port of its
//! `middleware/src/parse/*.ts` counterpart; goldens in `tests/golden/` are the oracle.

pub mod account;
pub mod availability;
pub mod decode;
pub mod featured;
pub mod picks;
pub mod record;
pub mod renew;
pub mod reserve;
pub mod search;
