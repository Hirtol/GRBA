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
mod branch;
mod load_store;
mod multi_load_store;

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

        // Hi register operations/branch exchange (TODO: Split on opcode, we can do that here!)
        // 0100_01XX
        if (i & 0xFC) == 0b0100_0100 {
            result[i] = ThumbV4::hi_reg_op_branch_exchange;
            continue;
        }

        // PC-Relative Load
        // 0100_1XXX
        if (i & 0xF8) == 0b0100_1000 {
            result[i] = ThumbV4::pc_relative_load;
            continue;
        }

        // Load/Store with Register Offset
        // 0101_XX0X
        if (i & 0xF2) == 0b0101_0000 {
            result[i] = ThumbV4::load_store_with_reg_offset;
            continue;
        }

        // Load/Store with Sign Extended Byte
        // 0101_XX1X
        if (i & 0xF2) == 0b0101_0010 {
            result[i] = ThumbV4::load_store_sign_extended_byte_halfword;
            continue;
        }

        // Load/Store with immediate offset
        // 011X_XXXX
        if (i & 0xE0) == 0b0110_0000 {
            result[i] = ThumbV4::load_store_with_immediate_offset;
            continue;
        }

        // Load/Store halfword
        // 1000_XXXX
        if (i & 0xF0) == 0b1000_0000 {
            result[i] = ThumbV4::load_store_halfword;
            continue;
        }

        // SP-relative load/store
        // 1001_XXXX
        if (i & 0xF0) == 0b1001_0000 {
            result[i] = ThumbV4::sp_relative_load_store;
            continue;
        }

        // Load address
        // 1010_XXXX
        if (i & 0xF0) == 0b1010_0000 {
            result[i] = ThumbV4::load_address;
            continue;
        }

        // Add offset to stack pointer
        // 1011_0000
        if (i & 0xFF) == 0b1011_0000 {
            result[i] = ThumbV4::add_offset_to_stack_pointer;
            continue;
        }

        // Push/Pop registers
        // 1011_X10X
        if (i & 0xF6) == 0b1011_0100 {
            result[i] = ThumbV4::push_pop_registers;
            continue;
        }
    }

    result
}
