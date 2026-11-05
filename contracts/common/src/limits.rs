//! Bounds that keep every method's gas and storage use predictable.

pub const MAX_ALLOCATIONS: usize = 10;
pub const MAX_TIERS: usize = 10;
pub const MAX_FIXED_SUPPLY: u32 = 10_000;
pub const MAX_MINT_PER_CALL: u32 = 10;
pub const MAX_PAGE_SIZE: u32 = 50;

pub const MAX_NAME_LEN: usize = 64;
pub const MAX_LABEL_LEN: usize = 32;
pub const MAX_DESCRIPTION_LEN: usize = 1_000;
pub const MAX_URL_LEN: usize = 512;
/// Inline `data:` icons are stored in contract state, so they are kept small.
pub const MAX_DATA_URI_LEN: usize = 8_192;

pub const MIN_SYMBOL_LEN: usize = 2;
pub const MAX_SYMBOL_LEN: usize = 12;
pub const MAX_DECIMALS: u8 = 24;

pub const MIN_SLUG_LEN: usize = 2;
pub const MAX_SLUG_LEN: usize = 20;
