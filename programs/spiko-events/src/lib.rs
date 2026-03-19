//! Shared event infrastructure for Spiko programs.
//!
//! Provides an Anchor-compatible `sol_log_data` wrapper and `emit_event` helper.
//! Events use the Anchor format: `SHA256("event:<EventName>")[0..8]` discriminator
//! followed by LE-packed field data.
//!
//! All serialization is manual LE packing into fixed-size stack buffers — no Borsh
//! dependency, matching the zero-copy philosophy of the rest of the codebase.

/// Emit a structured event via the `sol_log_data` syscall.
///
/// `data` should be a single byte slice: `discriminator(8) + LE-packed fields`.
/// This wraps `sol_log_data` with the correct `repr(C)` slice layout expected
/// by the Solana runtime.
///
/// On native (test) builds this is a no-op — `sol_log_data` is only available
/// when compiling for `target_os = "solana"`.
#[inline]
pub fn emit_event(data: &[u8]) {
    #[cfg(target_os = "solana")]
    {
        // sol_log_data expects an array of slices in repr(C) format:
        //   struct { ptr: *const u8, len: u64 }
        // We pass exactly one slice containing the full event payload.
        #[repr(C)]
        struct Slice {
            ptr: *const u8,
            len: u64,
        }

        let slices = [Slice {
            ptr: data.as_ptr(),
            len: data.len() as u64,
        }];

        unsafe {
            pinocchio::syscalls::sol_log_data(
                slices.as_ptr() as *const u8,
                1, // number of slices
            );
        }
    }

    // Suppress unused variable warning in native builds
    #[cfg(not(target_os = "solana"))]
    let _ = data;
}

// ---------------------------------------------------------------
// Buffer packing helpers
// ---------------------------------------------------------------

/// Write a 32-byte address into `buf` at `offset`. Returns new offset.
#[inline(always)]
pub fn pack_address(buf: &mut [u8], offset: usize, addr: &[u8; 32]) -> usize {
    buf[offset..offset + 32].copy_from_slice(addr);
    offset + 32
}

/// Write a u64 (LE) into `buf` at `offset`. Returns new offset.
#[inline(always)]
pub fn pack_u64(buf: &mut [u8], offset: usize, val: u64) -> usize {
    buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
    offset + 8
}

/// Write an i64 (LE) into `buf` at `offset`. Returns new offset.
#[inline(always)]
pub fn pack_i64(buf: &mut [u8], offset: usize, val: i64) -> usize {
    buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
    offset + 8
}

/// Write a u8 into `buf` at `offset`. Returns new offset.
#[inline(always)]
pub fn pack_u8(buf: &mut [u8], offset: usize, val: u8) -> usize {
    buf[offset] = val;
    offset + 1
}

/// Write the 8-byte discriminator into `buf` at offset 0. Returns 8.
#[inline(always)]
pub fn pack_disc(buf: &mut [u8], disc: &[u8; 8]) -> usize {
    buf[0..8].copy_from_slice(disc);
    8
}
