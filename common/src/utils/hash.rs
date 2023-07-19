pub use ckb_hash::blake2b_256 as ckb_blake2b_256;

use tiny_keccak::{Hasher, Keccak};

pub fn keccak256<B: AsRef<[u8]>>(data: B) -> [u8; 32] {
    let mut result = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(data.as_ref());
    hasher.finalize(&mut result);
    result
}
