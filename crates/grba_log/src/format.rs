use zerocopy::{ByteSlice, LayoutVerified};

/// The format from the Logs that we have from other emulators.
/// Should really use `U32<LittleEndian>`, but that doesn't implement hex debug print :(
#[derive(zerocopy::FromBytes, Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
#[repr(C)]
pub struct InstructionSnapshot {
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
    r4: u32,
    r5: u32,
    r6: u32,
    r7: u32,
    r8: u32,
    r9: u32,
    r10: u32,
    r11: u32,
    r12: u32,
    r13: u32,
    r14: u32,
    r15: u32,
    cpsr: u32,
    spsr: u32,
}

impl InstructionSnapshot {
    pub fn parse<B: ByteSlice>(bytes: B) -> Option<LayoutVerified<B, [Self]>> {
        let result = zerocopy::LayoutVerified::new_slice(bytes)?;

        Some(result)
    }
}
