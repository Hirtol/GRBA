use crate::emulator::bus::Bus;
use crate::emulator::cpu::CPU;

/// For indexing into the LUT we use a 8-bit value, which is derived from a bitmasked instruction.
pub const THUMB_LUT_SIZE: usize = 256;

pub type ThumbInstruction = u16;
pub type LutInstruction = fn(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus);
/// The LUT is a lookup table of instructions.
/// The index is derived from the instruction, namely the upper byte of the [ThumbInstruction]
pub type ThumbLUT = [LutInstruction; THUMB_LUT_SIZE];

mod alu;

/// Contains all Thumb instructions for the ArmV4T.
pub struct ThumbV4;

impl ThumbV4 {}

pub(crate) fn create_thumb_lut() -> ThumbLUT {
    fn dead_fn(_cpu: &mut CPU, instruction: ThumbInstruction, _bus: &mut Bus) {
        panic!("Unimplemented thumb instruction: {:#08X}", instruction);
    }

    let mut result: ThumbLUT = [dead_fn as LutInstruction; THUMB_LUT_SIZE];

    for i in 0..THUMB_LUT_SIZE {
        // Move Shifted Register:
        // 000X_XXXX
        if (i & 0xE0) == 0b0000_0000 {
            result[i] = ThumbV4::move_shifted_reg;
            continue;
        }

        // Add/Subtract
        // 0001_1XXX
        if (i & 0xF8) == 0b0001_1000 {
            //TODO: Split on Opcode/Immediate value
            result[i] = ThumbV4::add_subtract;
            continue;
        }

        // move/compare/add/subtract immediate
        // 001X_XXXX
        if (i & 0xE0) == 0b0010_0000 {
            result[i] = ThumbV4::move_compare_add_subtract;
            continue;
        }

        // ALU operations
        // 0100_00XX
        if (i & 0xFC) == 0b0100_0000 {
            result[i] = ThumbV4::alu_operations;
            continue;
        }
    }

    result
}
