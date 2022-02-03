use crate::{check_bit, get_bits};

#[inline(always)]
pub const fn check_bit(value: u32, bit: u8) -> bool {
    check_bit!(value, bit)
}

#[inline(always)]
pub const fn check_bit_64(value: u64, bit: u8) -> bool {
    check_bit!(value, bit)
}

/// Return the bits in the specified range.
/// Will be optimised by the compiler to a simple `shift` and `and`.
///
/// ```ignore
///
/// // Get bits in the range of 12..=15
/// let value = get_bits(0xBEEF, 12, 15);
///
/// assert_eq!(value, 0xB);
/// ```
#[inline(always)]
pub const fn get_bits(val: u32, begin: u32, end_inclusive: u32) -> u32 {
    get_bits!(val, begin, end_inclusive)
}

/// Check if a sign overflow occurred
/// TODO: Verify if this is correct
#[inline(always)]
pub const fn has_sign_overflowed(val1: u32, val2: u32, result: u32) -> bool {
    ((val1 ^ result) & (!val2 ^ result)) >> 31 != 0
}

#[cfg(test)]
mod tests {
    use crate::utils::get_bits;

    #[test]
    pub fn get_bits_test() {
        let val = 0xBEEF;

        assert_eq!(get_bits(val, 12, 15), 0xB);
        assert_eq!(get_bits(val, 8, 11), 0xE);
        assert_eq!(get_bits(val, 4, 7), 0xE);
        assert_eq!(get_bits(val, 0, 3), 0xF);

        assert_eq!(get_bits(val, 1, 4), 0x7);
    }
}
