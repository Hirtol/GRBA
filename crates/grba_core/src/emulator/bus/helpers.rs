use crate::emulator::{AlignedAddress, MemoryAddress};

/// Check whether the provided two types are equivalent
///
/// # Examples
/// ```no_run
/// assert!(grba_core::is_same_type!(u8, u8));
/// ```
#[macro_export]
macro_rules! is_same_type {
    ($a:ty, $b:ty) => {
        ::core::any::TypeId::of::<$a>() == ::core::any::TypeId::of::<$b>()
    };
}

pub trait ReadType {
    /// Read self from the provided slice.
    ///
    /// The slice should be equivalent in size to `std::mem::size_of::<Self>()`
    fn from_le_bytes(bytes: &[u8]) -> Self;

    /// Align the provided address to the appropriate boundary for this current type
    ///
    /// # Alignment
    /// * `u32` - 4-byte alignment (aka, `address & 0xFFFF_FFFC`)
    /// * `u16` - 2-byte alignment (aka, `address & 0xFFFF_FFFE`)
    /// * `u8`  - Any
    fn align_address(address: MemoryAddress) -> AlignedAddress;
}

macro_rules! impl_readable_type {
    ($($arg:tt),*) => {
        $(
        impl ReadType for $arg {

            #[inline(always)]
            fn from_le_bytes(bytes: &[u8]) -> Self {
                use std::convert::TryInto;
                $arg::from_le_bytes(bytes.try_into().unwrap())
            }

            #[inline(always)]
            fn align_address(address: MemoryAddress) -> AlignedAddress {
                address & !(std::mem::size_of::<Self>() as MemoryAddress - 1)
            }
        })*
    };
}

impl_readable_type!(u8, u16, u32);
