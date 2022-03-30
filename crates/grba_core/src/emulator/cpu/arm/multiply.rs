use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmV4};
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;

impl ArmV4 {
    /// Implements the `MUL` and `MLA` instructions.
    pub fn multiply(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        let accumulate = instruction.check_bit(21);
        let should_set_condition = instruction.check_bit(20);
        let r_d = instruction.get_bits(16, 19) as usize;
        let r_add = instruction.get_bits(12, 15) as usize;
        let r_1 = instruction.get_bits(8, 11) as usize;
        let r_2 = instruction.get_bits(0, 3) as usize;

        // Check if the accumulate flag is set by casting it to u32, and then adding.
        // Doing this elides a branch (Sadly, compiler doesn't do it for us according to GodBolt :( )
        let result = cpu
            .read_reg(r_1)
            .wrapping_mul(cpu.read_reg(r_2))
            .wrapping_add(accumulate as u32 * cpu.read_reg(r_add));
        cpu.write_reg(r_d, result, bus);

        if should_set_condition {
            cpu.set_zero_and_sign(result);
            // Carry flag set to meaningless value?
        }
    }

    pub fn multiply_long(cpu: &mut CPU, instruction: ArmInstruction, _bus: &mut Bus) {
        let signed = instruction.check_bit(22);
        let accumulate = instruction.check_bit(21);
        let should_set_condition = instruction.check_bit(20);
        let r_high = instruction.get_bits(16, 19) as usize;
        let r_low = instruction.get_bits(12, 15) as usize;
        let r_1 = instruction.get_bits(8, 11) as usize;
        let r_2 = instruction.get_bits(0, 3) as usize;

        if signed {
            ArmV4::multiply_long_signed(cpu, accumulate, should_set_condition, r_high, r_low, r_1, r_2);
        } else {
            ArmV4::multiply_long_unsigned(cpu, accumulate, should_set_condition, r_high, r_low, r_1, r_2);
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
                + (((registers[reg_high] as u64) << 32) | registers[reg_low] as u64)
        } else {
            registers[reg_1] as u64 * registers[reg_2] as u64
        };

        registers[reg_high] = (result >> 32) as u32;
        registers[reg_low] = result as u32;

        if should_set_condition {
            cpu.registers.cpsr.set_sign(result.check_bit(63));
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
            // Note that we purposefully don't sign extend here, if the lower register would be sign-extended it could
            // overwrite the high-register's value during the bitwise-or.
            let addition = (((registers[reg_high] as i64) << 32) | registers[reg_low] as i64);

            (registers[reg_1] as i32 as i64 * registers[reg_2] as i32 as i64) + addition
        } else {
            registers[reg_1] as i32 as i64 * registers[reg_2] as i32 as i64
        };

        registers[reg_high] = (result >> 32) as u32;
        registers[reg_low] = result as u32;

        if should_set_condition {
            cpu.registers.cpsr.set_sign(result.check_bit(63));
            cpu.registers.cpsr.set_zero(result == 0);
            // Carry and overflow flags set to meaningless value?
        }
    }
}
