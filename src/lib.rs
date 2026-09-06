#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod bitarray;
pub mod channel;
pub mod encoder;
pub mod ldpc_decoder;
pub mod matrices;
#[cfg(feature = "std")]
pub mod server_router;
pub mod spa_decoder_llr;
