use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Global monotonic counter to guarantee per-call uniqueness even when wall-clock
/// time resolution is coarse.
static RUNTIME_SEED_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Fallback non-zero seed used only if mixed entropy unexpectedly becomes zero.
const NON_ZERO_FALLBACK_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// Returns a best-effort non-zero runtime entropy seed.
///
/// This is intended for gameplay randomness (match/wave/item spawn streams), not
/// cryptographic use. It intentionally mixes:
/// - current time,
/// - process id,
/// - thread id,
/// - a monotonic per-call counter,
/// - a small amount of address entropy.
///
/// The monotonic counter prevents repeated seeds when multiple systems reseed
/// in the same timestamp window.
pub fn runtime_entropy_seed_u64() -> u64 {
    let now_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0);

    let counter = RUNTIME_SEED_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = process::id() as u64;
    let tid = thread_id_hash_u64();

    // Minor address entropy (non-stable by design; useful for runtime variance).
    let stack_anchor = &counter as *const u64 as usize as u64;

    let mut seed = NON_ZERO_FALLBACK_SEED;
    seed ^= splitmix64_finalize(now_nanos);
    seed ^= splitmix64_finalize(pid.rotate_left(17) ^ counter.rotate_left(29));
    seed ^= splitmix64_finalize(tid.rotate_left(11) ^ stack_anchor.rotate_left(41));
    seed = splitmix64_finalize(seed ^ counter.wrapping_mul(0xD134_2543_DE82_EF95));

    non_zero_seed(seed)
}

/// Convenience 32-bit runtime seed derived from [`runtime_entropy_seed_u64`].
pub fn runtime_entropy_seed_u32() -> u32 {
    let value = runtime_entropy_seed_u64();
    let folded = ((value >> 32) as u32) ^ (value as u32);
    if folded == 0 {
        (NON_ZERO_FALLBACK_SEED as u32) ^ ((NON_ZERO_FALLBACK_SEED >> 32) as u32)
    } else {
        folded
    }
}

/// Deterministically derives a non-zero stream seed from a base seed and stream tag.
///
/// Useful for domain-separated RNG streams, e.g.:
/// - match seed -> wave seed
/// - wave seed -> enemy spawn seed
/// - match seed -> rescue/drop/upgrade seed
pub fn derive_stream_seed(base_seed: u64, stream_tag: u64) -> u64 {
    let mixed = splitmix64_finalize(
        base_seed
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ stream_tag.wrapping_mul(0xD1B5_4A32_D192_ED03)
            ^ (base_seed ^ stream_tag).rotate_left(27),
    );
    non_zero_seed(mixed)
}

/// Normalizes a provided seed into a non-zero RNG state.
pub fn non_zero_seed(seed: u64) -> u64 {
    if seed == 0 {
        NON_ZERO_FALLBACK_SEED
    } else {
        seed
    }
}

fn thread_id_hash_u64() -> u64 {
    let mut hasher = DefaultHasher::new();
    std::thread::current().id().hash(&mut hasher);
    hasher.finish()
}

/// SplitMix64 finalizer (fast, high-quality bit diffusion for non-crypto use).
fn splitmix64_finalize(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    use super::{derive_stream_seed, non_zero_seed, runtime_entropy_seed_u64};

    #[test]
    fn runtime_seed_is_non_zero() {
        assert_ne!(runtime_entropy_seed_u64(), 0);
    }

    #[test]
    fn consecutive_runtime_seeds_differ() {
        let a = runtime_entropy_seed_u64();
        let b = runtime_entropy_seed_u64();
        assert_ne!(a, b);
    }

    #[test]
    fn derived_stream_seed_is_deterministic_for_same_inputs() {
        let base = 0x1234_5678_ABCD_EF01;
        let tag = 0x00FE_DCBA_9876_5432;
        let first = derive_stream_seed(base, tag);
        let second = derive_stream_seed(base, tag);
        assert_eq!(first, second);
    }

    #[test]
    fn derived_stream_seed_changes_with_tag() {
        let base = 0x1234_5678_ABCD_EF01;
        let left = derive_stream_seed(base, 1);
        let right = derive_stream_seed(base, 2);
        assert_ne!(left, right);
    }

    #[test]
    fn non_zero_seed_replaces_zero() {
        assert_ne!(non_zero_seed(0), 0);
        assert_eq!(non_zero_seed(42), 42);
    }
}