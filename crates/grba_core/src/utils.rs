use crate::{check_bit, get_bits};

/// Check if a sign overflow occurred
/// TODO: Verify if this is correct
#[inline(always)]
pub const fn has_sign_overflowed(val1: u32, val2: u32, result: u32) -> bool {
    ((val1 ^ result) & (!val2 ^ result)) >> 31 != 0
}

#[inline(always)]
pub fn sign_extend32(data: u32, size: u8) -> i32 {
    ((data << (32 - size)) as i32) >> (32 - size)
}

pub trait BitOps {
    type Output;
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
    fn get_bits(self, begin: u8, end_inclusive: u8) -> Self::Output;

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
}

macro_rules! impl_bitops {
    ($($t:ty),*) => {
        $(
            impl BitOps for $t {
                type Output = $t;

                #[inline(always)]
                fn get_bits(self, begin: u8, end_inclusive: u8) -> $t {
                    get_bits!(self, begin, end_inclusive)
                }

                #[inline(always)]
                fn check_bit(self, bit: u8) -> bool {
                    check_bit!(self, bit)
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
        // Use a generic function so that the pointer cast remains type-safe
        fn vec_to_boxed_array<T>(vec: Vec<T>) -> Box<[T; $len]> {
            let boxed_slice = vec.into_boxed_slice();

            let ptr = ::std::boxed::Box::into_raw(boxed_slice) as *mut [T; $len];

            unsafe { Box::from_raw(ptr) }
        }

        vec_to_boxed_array(vec![$val; $len])
    }};
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
