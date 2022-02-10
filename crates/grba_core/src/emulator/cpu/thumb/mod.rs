use crate::emulator::bus::Bus;
use crate::emulator::cpu::CPU;

/// For indexing into the LUT we use a 8-bit value, which is derived from a bitmasked instruction.
pub const THUMB_LUT_SIZE: usize = 256;

pub type ThumbInstruction = u16;
pub type LutInstruction = fn(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus);
pub type ThumbLUT = [LutInstruction; THUMB_LUT_SIZE];

mod alu;

/// Contains all Thumb instructions for the ArmV4T.
pub struct ThumbV4;

impl ThumbV4 {}

pub(crate) fn create_thumb_lut() -> ThumbLUT {
    fn dead_fn(_cpu: &mut CPU, instruction: ThumbInstruction, _bus: &mut Bus) {
        panic!("Unimplemented thumb instruction: {:08x}", instruction);
    }

    let mut result: ThumbLUT = [dead_fn as LutInstruction; THUMB_LUT_SIZE];

    for i in 0..THUMB_LUT_SIZE {
        result[i] = dead_fn;
    }

    result
}
