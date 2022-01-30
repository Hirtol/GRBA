#[inline(always)]
pub const fn check_bit(value: u32, bit: u8) -> bool {
    (value & (1 << bit)) != 0
}

#[inline(always)]
pub const fn check_bit_64(value: u64, bit: u8) -> bool {
    (value & (1 << bit)) != 0
}
