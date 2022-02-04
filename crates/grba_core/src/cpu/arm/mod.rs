use crate::bus::Bus;
use crate::cpu::CPU;
use crate::utils::{check_bit, check_bit_64, get_bits};

/// For indexing into the LUT we use a 12-bit value, which is derived from a bitmasked instruction.
pub const ARM_LUT_SIZE: usize = 4096;

pub type ArmInstruction = u32;
pub type LutInstruction = fn(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus);
pub type ArmLUT = [LutInstruction; ARM_LUT_SIZE];

mod data_processing;
mod psr_transfer;
mod single_data_swap;

pub struct ArmV4T;

impl ArmV4T {
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

    /// Implements the `MUL` and `MLA` instructions.
    pub fn multiply(cpu: &mut CPU, instruction: ArmInstruction, _bus: &mut Bus) {
        //TODO: Instruction timing
        let accumulate = check_bit(instruction, 21);
        let should_set_condition = check_bit(instruction, 20);
        let (reg_destination, reg_add) = get_high_registers(instruction);
        let reg_1 = ((instruction >> 8) & 0xF) as usize;
        let reg_2 = (instruction & 0xF) as usize;

        // Check if the accumulate flag is set by casting it to u32, and then adding.
        // Doing this elides a branch (Sadly, compiler doesn't do it for us according to GodBolt :( )
        let result = cpu
            .read_reg(reg_1)
            .wrapping_mul(cpu.read_reg(reg_2))
            .wrapping_add(accumulate as u32 * cpu.read_reg(reg_add));
        cpu.write_reg(reg_destination, result);

        if should_set_condition {
            cpu.registers.cpsr.set_sign(check_bit(result, 31));
            cpu.registers.cpsr.set_zero(result == 0);
            // Carry flag set to meaningless value?
        }
    }

    pub fn multiply_long(cpu: &mut CPU, instruction: ArmInstruction, _bus: &mut Bus) {
        //TODO: Instruction timing
        let unsigned = check_bit(instruction, 22);
        let accumulate = check_bit(instruction, 21);
        let should_set_condition = check_bit(instruction, 20);
        let (reg_high, reg_low) = get_high_registers(instruction);
        let reg_1 = ((instruction >> 8) & 0xF) as usize;
        let reg_2 = (instruction & 0xF) as usize;
        //TODO: Can probably just cast the signed result to a u64 and keep all logic in this function, `as u64` should
        // only re-interpret the bits as a u64.
        if unsigned {
            ArmV4T::multiply_long_unsigned(cpu, accumulate, should_set_condition, reg_high, reg_low, reg_1, reg_2);
        } else {
            ArmV4T::multiply_long_signed(cpu, accumulate, should_set_condition, reg_high, reg_low, reg_1, reg_2);
        }
    }

    fn multiply_long_unsigned(
        cpu: &mut CPU,
        accumulate: bool,
        should_set_condition: bool,
        reg_high: usize,
        reg_low: usize,
        reg_1: usize,
        reg_2: usize,
    ) {
        let registers = &mut cpu.registers.general_purpose;
        let result = if accumulate {
            registers[reg_1] as u64 * registers[reg_2] as u64
        } else {
            registers[reg_1] as u64 * registers[reg_2] as u64
                + (((registers[reg_high] as u64) << 32) | registers[reg_low] as u64)
        };

        registers[reg_high] = (result >> 32) as u32;
        registers[reg_low] = result as u32;

        if should_set_condition {
            cpu.registers.cpsr.set_sign(check_bit_64(result, 63));
            cpu.registers.cpsr.set_zero(result == 0);
            // Carry and overflow flags set to meaningless value?
        }
    }

    fn multiply_long_signed(
        cpu: &mut CPU,
        accumulate: bool,
        should_set_condition: bool,
        reg_high: usize,
        reg_low: usize,
        reg_1: usize,
        reg_2: usize,
    ) {
        let registers = &mut cpu.registers.general_purpose;
        let result = if accumulate {
            registers[reg_1] as i32 as i64 * registers[reg_2] as i32 as i64
        } else {
            registers[reg_1] as i32 as i64 * registers[reg_2] as i32 as i64
                + (((registers[reg_high] as i32 as i64) << 32) | registers[reg_low] as i32 as i64)
        };

        registers[reg_high] = (result >> 32) as u32;
        registers[reg_low] = result as u32;

        if should_set_condition {
            cpu.registers.cpsr.set_sign(check_bit_64(result as u64, 63));
            cpu.registers.cpsr.set_zero(result == 0);
            // Carry and overflow flags set to meaningless value?
        }
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
        // Multiply:
        // 0000_00XX_1001
        if (i & 0xFCF) == 0b0000_0000_1001 {
            result[i] = ArmV4T::multiply;
            continue;
        }

        // Multiply long:
        // 0000_1XXX_1001
        if (i & 0xF8F) == 0b0000_1000_1001 {
            result[i] = ArmV4T::multiply_long;
            continue;
        }

        // MRS (Transfer PSR to register):
        // 0001_0X00_0000
        if (i & 0xFBF) == 0b0001_0000_0000 {
            result[i] = ArmV4T::mrs_trans_psr_reg;
            continue;
        }

        // MSR (Transfer register to PSR Condition Flags):
        // 00X1_0X10_XXXX
        if (i & 0xDB0) == 0b0001_0010_0000 {
            result[i] = ArmV4T::msr_match;
            continue;
        }

        // Data Processing:
        // 00XX_XXXX_XXXX
        if (i & 0xC00) == 0b0000_0000_0000 {
            result[i] = ArmV4T::data_processing;
            continue;
        }
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

#[inline(always)]
pub(crate) fn get_low_register(instruction: ArmInstruction) -> usize {
    (instruction & 0xF) as usize
}

#[cfg(test)]
mod tests {
    use crate::cpu::arm::ArmV4T;

    #[test]
    fn test_lut_filling() {
        let lut = super::create_arm_lut();

        // Check MSR matching
        let fn_ref = lut[0b0011_0110_0000];

        assert_eq!(fn_ref as usize, ArmV4T::msr_match as usize);

        // Check Data Processing matching (AND operation in immediate mode)
        let fn_ref = lut[0b0010_0000_0000];

        assert_eq!(fn_ref as usize, ArmV4T::data_processing as usize);
    }
}
