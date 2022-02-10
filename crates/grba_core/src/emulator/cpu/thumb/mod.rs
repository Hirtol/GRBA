use crate::emulator::bus::Bus;
use crate::emulator::cpu::CPU;

/// For indexing into the LUT we use a 8-bit value, which is derived from a bitmasked instruction.
pub const THUMB_LUT_SIZE: usize = 256;

pub type ThumbInstruction = u32;
pub type LutInstruction = fn(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus);
pub type ThumbLUT = [LutInstruction; THUMB_LUT_SIZE];

/// Contains all Thumb instructions for the ArmV4T.
pub struct ThumbV4;

impl ThumbV4 {}
