pub const fn check_bit(value: u32, bit: u8) -> bool {
    (value & (1 << bit)) != 0
}
