use crate::emulator::bus::Bus;
use crate::emulator::cpu::{Exception, CPU};
use crate::utils::BitOps;

/// For indexing into the LUT we use a 12-bit value, which is derived from a bitmasked instruction.
pub const ARM_LUT_SIZE: usize = 4096;

pub type ArmInstruction = u32;
pub type LutInstruction = fn(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus);
pub type ArmLUT = [LutInstruction; ARM_LUT_SIZE];

mod block_data_transfer;
mod branching;
mod data_processing;
mod load_store;
mod multiply;
mod psr_transfer;
mod single_data_swap;

pub struct ArmV4;

impl ArmV4 {
    /// Check if the conditional flag set in the provided `instruction` holds.
    pub fn condition_holds(cpu: &CPU, instruction: ArmInstruction) -> bool {
        // Upper 4 bits contain the condition code for all ARM instructions
        let flags = instruction >> 28;
        let cpsr = &cpu.registers.cpsr;

        match flags {
            // Is zero set (is equal)
            0b0000 => cpsr.zero(),
            // Is zero not set (not equal)
            0b0001 => !cpsr.zero(),
            // Is carry
            0b0010 => cpsr.carry(),
            // Is carry clear
            0b0011 => !cpsr.carry(),
            // Is sign negative
            0b0100 => cpsr.sign(),
            // Is sign positive or zero
            0b0101 => !cpsr.sign(),
            // Has overflowed
            0b0110 => cpsr.overflow(),
            // No overflow
            0b0111 => !cpsr.overflow(),
            0b1000 => cpsr.carry() && !cpsr.zero(),
            0b1001 => !cpsr.carry() && cpsr.zero(),
            // Greater than or equal
            0b1010 => cpsr.sign() == cpsr.overflow(),
            // Less than
            0b1011 => cpsr.sign() != cpsr.overflow(),
            // Greater than
            0b1100 => !cpsr.zero() && (cpsr.sign() == cpsr.overflow()),
            // Less than or equal
            0b1101 => cpsr.zero() || (cpsr.sign() != cpsr.overflow()),
            // Always
            0b1110 => true,
            // Never
            0b1111 => false,
            _ => panic!("Impossible condition code, did the bit shift get changed?"),
        }
    }

    pub fn undefined_instruction(cpu: &mut CPU, _instruction: ArmInstruction, bus: &mut Bus) {
        cpu.raise_exception(bus, Exception::UndefinedInstruction)
    }

    pub fn software_interrupt(cpu: &mut CPU, _instruction: ArmInstruction, bus: &mut Bus) {
        cpu.raise_exception(bus, Exception::SoftwareInterrupt)
    }
}

/// Create a lookup table for the ARM instructions.
///
/// Would be a `const fn` if stable had const function pointers.
/// Assumes `12-bit` indexing
pub(crate) fn create_arm_lut() -> ArmLUT {
    fn dead_fn(_cpu: &mut CPU, instruction: ArmInstruction, _bus: &mut Bus) {
        panic!("Unimplemented instruction: {:08x}", instruction);
    }

    let mut result = [dead_fn as LutInstruction; 4096];

    for i in 0..ARM_LUT_SIZE {
        // Software Interrupt:
        // 1111_XXXX_XXXX
        if (i & 0xF00) == 0b1111_0000_0000 {
            result[i] = ArmV4::software_interrupt;
            continue;
        }

        // Block Data Transfer:
        // 100X_XXXX_XXXX
        if (i & 0xE00) == 0b1000_0000_0000 {
            // Check load bit ahead of time.
            if i.check_bit(4) {
                result[i] = ArmV4::block_data_transfer_load;
            } else {
                result[i] = ArmV4::block_data_transfer_store;
            }
            continue;
        }

        // Multiply:
        // 0000_00XX_1001
        if (i & 0xFCF) == 0b0000_0000_1001 {
            result[i] = ArmV4::multiply;
            continue;
        }

        // Multiply long:
        // 0000_1XXX_1001
        if (i & 0xF8F) == 0b0000_1000_1001 {
            result[i] = ArmV4::multiply_long;
            continue;
        }

        {
            // This is one block, as single data swap should always be filled in before the halfword transfer (as it is
            // a part of their matching).

            // Single Data Swap:
            // 0001_0X00_1001
            if (i & 0xFBF) == 0b0001_0000_1001 {
                result[i] = ArmV4::single_data_swap;
                continue;
            }

            // Halfword Data Transfer, register:
            // 000X_X0XX_1XX1
            if (i & 0xE49) == 0b0000_0000_1001 {
                result[i] = ArmV4::halfword_and_signed_register;
                continue;
            }

            // Halfword Data Transfer, immediate:
            // 000X_X1XX_1XX1
            if (i & 0xE49) == 0b0000_0100_1001 {
                result[i] = ArmV4::halfword_and_signed_immediate;
                continue;
            }
        }

        // Branch and Exchange:
        // 0001_0010_0001
        if i == 0b0001_0010_0001 {
            result[i] = ArmV4::branch_and_exchange;
            continue;
        }

        // Branch:
        // 101X_XXXX_XXXX
        if (i & 0xE00) == 0b1010_0000_0000 {
            result[i] = ArmV4::branch_and_link;
            continue;
        }

        {
            // TODO: A little confused by the undefined instruction, as it seems to overlap with single data transfer
            // Single Data Transfer:
            // 01XX_XXXX_XXXX
            if (i & 0xC00) == 0b0100_0000_0000 {
                result[i] = ArmV4::single_data_transfer;
                continue;
            }
        }

        // MRS (Transfer PSR to register):
        // 0001_0X00_0000
        if (i & 0xFBF) == 0b0001_0000_0000 {
            result[i] = ArmV4::mrs_trans_psr_reg;
            continue;
        }

        // MSR (Transfer register to PSR Condition Flags):
        // 00X1_0X10_XXXX
        if (i & 0xDB0) == 0b0001_0010_0000 {
            result[i] = ArmV4::msr_match;
            continue;
        }

        // Data Processing Immediate:
        // 001X_XXXX_XXXX
        if (i & 0xE00) == 0b0010_0000_0000 {
            result[i] = ArmV4::data_processing_immediate;
            continue;
        }

        // Data Processing Register:
        // 000X_XXXX_XXXX
        if (i & 0xE00) == 0b0000_0000_0000 {
            // Check the shift type
            let is_register_shift = i.check_bit(0);

            if is_register_shift {
                result[i] = ArmV4::data_processing_register_register_shift;
            } else {
                result[i] = ArmV4::data_processing_register_immediate_shift;
            }

            continue;
        }

        // Any remaining will be undefined
        result[i] = ArmV4::undefined_instruction;
    }

    result
}

/// Returns the two most significant bit registers of an instruction.
/// Since all ARM instructions follow this kind of format:
/// * `0000_0000_0000_XXXX_YYYY_0000_0000_ZZZZ`
///
/// This function will return `(XXXX, YYYY)`
/// See [get_low_registers] for `ZZZZ`.
#[inline(always)]
pub(crate) fn get_high_registers(instruction: ArmInstruction) -> (usize, usize) {
    let rn = ((instruction >> 16) & 0xF) as usize;
    let rd = ((instruction >> 12) & 0xF) as usize;
    (rn, rd)
}

#[cfg(test)]
mod tests {
    use crate::emulator::cpu::arm::ArmV4;

    #[test]
    fn test_lut_filling() {
        let lut = super::create_arm_lut();

        // Check MSR matching
        let fn_ref = lut[0b0011_0110_0000];

        assert_eq!(fn_ref as usize, ArmV4::msr_match as usize);

        // Check Data Processing matching (AND operation in immediate mode)
        let fn_ref = lut[0b0010_0000_0000];

        assert_eq!(fn_ref as usize, ArmV4::data_processing_immediate as usize);
    }
}
