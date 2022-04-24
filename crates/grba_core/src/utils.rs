use std::fmt::Debug;

/// Check if a sign overflow occurred
#[inline(always)]
pub const fn has_sign_overflowed(val1: u32, val2: u32, result: u32) -> bool {
    ((val1 ^ result) & (val2 ^ result)) >> 31 != 0
}

#[inline(always)]
pub const fn sign_extend32(data: u32, size: u8) -> i32 {
    ((data << (32 - size)) as i32) >> (32 - size)
}

pub trait BitOps {
    /// Return the bits in the specified range.
    /// Will be optimised by the compiler to a simple `shift` and `and`.
    ///
    /// ```ignore
    ///
    /// // Get bits in the range of 12..=15
    /// let value = 0xBEEF.get_bits(12, 15);
    ///
    /// assert_eq!(value, 0xB);
    /// ```
    fn get_bits(self, begin: u8, end_inclusive: u8) -> Self;

    /// Check the provided bit, if it's set return `true`, otherwise return `false`.
    ///
    /// ```ignore
    ///
    /// // Check if the last bit is set
    /// let set = 0xBEEF.check_bit(15);
    ///
    /// assert!(set);
    /// ```
    fn check_bit(self, bit: u8) -> bool;

    /// Update the provided `byte` in `self` to the specified `value`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use grba_core::utils::BitOps;
    /// let mut value: u16 = 0xBEEF;
    /// let new_value = value.change_byte_le(0, 0xBE);
    ///
    /// assert_eq!(new_value, 0xBEBE);
    /// ```
    fn change_byte_le(self, byte: usize, value: u8) -> Self;

    /// Update the provided range `begin..=end_inclusive` in `self` to the specified `value`.
    fn change_bits(self, begin: u8, end_inclusive: u8, value: Self) -> Self;
}

macro_rules! impl_bitops {
    ($($t:ty),*) => {
        $(
            impl BitOps for $t {
                #[inline(always)]
                fn get_bits(self, begin: u8, end_inclusive: u8) -> $t {
                    (self >> begin) & ((1 << (end_inclusive - begin + 1)) - 1)
                }

                #[inline(always)]
                fn check_bit(self, bit: u8) -> bool {
                    (self & (1 << bit)) != 0
                }

                #[inline(always)]
                fn change_byte_le(self, byte: usize, value: u8) -> $t {
                    let mut bytes = self.to_le_bytes();
                    bytes[byte] = value;
                    Self::from_le_bytes(bytes)
                }

                #[inline(always)]
                fn change_bits(self, begin: u8, end_inclusive: u8, value: Self) -> $t {
                    let mask = ((1 << (end_inclusive - begin + 1)) - 1) << begin;
                    (self & !mask) | (value << begin)
                }
            }
        )*
    };
}

impl_bitops!(u8, u16, u32, u64, i8, i16, i32, i64, usize, isize);

/// A macro similar to `vec![$elem; $size]` which returns a boxed array.
///
/// ```rustc
///     let _: Box<[u8; 1024]> = box_array![0; 1024];
/// ```
#[macro_export]
macro_rules! box_array {
    ($val:expr ; $len:expr) => {{
        $crate::utils::alloc_array::<_, { $len }>($val)
    }};
}

/// Allocate a sized array on the heap, without first creating a stack-allocated array.
///
/// Useful for large allocations which would otherwise overflow the stack.
pub fn alloc_array<T: Clone + Debug, const N: usize>(default_val: T) -> Box<[T; N]> {
    vec![default_val; N]
        .into_boxed_slice()
        .try_into()
        .expect("Incorrect array size")
}

#[cfg(test)]
mod tests {
    use crate::utils::BitOps;

    #[test]
    pub fn get_bits_test() {
        let val = 0xBEEF;

        assert_eq!(val.get_bits(12, 15), 0xB);
        assert_eq!(val.get_bits(8, 11), 0xE);
        assert_eq!(val.get_bits(4, 7), 0xE);
        assert_eq!(val.get_bits(0, 3), 0xF);

        assert_eq!(val.get_bits(1, 4), 0x7);
    }
}
