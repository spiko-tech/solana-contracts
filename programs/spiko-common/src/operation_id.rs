/// Build the 80-byte hash input: user(32) || mint(32) || amount(8 LE) || salt(8 LE)
#[inline]
fn build_hash_input(user: &[u8; 32], token_mint: &[u8; 32], amount: u64, salt: u64) -> [u8; 80] {
    let mut input = [0u8; 80];
    input[0..32].copy_from_slice(user);
    input[32..64].copy_from_slice(token_mint);
    input[64..72].copy_from_slice(&amount.to_le_bytes());
    input[72..80].copy_from_slice(&salt.to_le_bytes());
    input
}

/// Compute operation_id = SHA256(user || mint || amount_le || salt_le)
///
/// On-chain: uses Solana's sol_sha256 syscall.
/// Native (tests): uses the `sha2` crate.
#[cfg(target_os = "solana")]
pub fn compute_operation_id(
    user: &[u8; 32],
    token_mint: &[u8; 32],
    amount: u64,
    salt: u64,
) -> [u8; 32] {
    let input = build_hash_input(user, token_mint, amount, salt);
    let mut hash_result = [0u8; 32];

    #[repr(C)]
    struct Slice {
        ptr: *const u8,
        len: u64,
    }

    let slices = [Slice {
        ptr: input.as_ptr(),
        len: 80,
    }];

    unsafe {
        pinocchio::syscalls::sol_sha256(slices.as_ptr() as *const u8, 1, hash_result.as_mut_ptr());
    }

    hash_result
}

/// Native fallback for tests — uses the `sha2` crate.
#[cfg(not(target_os = "solana"))]
pub fn compute_operation_id(
    user: &[u8; 32],
    token_mint: &[u8; 32],
    amount: u64,
    salt: u64,
) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let input = build_hash_input(user, token_mint, amount, salt);
    let result = Sha256::digest(&input);
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}
