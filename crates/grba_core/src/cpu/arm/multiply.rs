use crate::bus::Bus;
use crate::cpu::arm::{ArmInstruction, ArmV4T};
use crate::cpu::{arm, CPU};
use crate::utils::BitOps;

impl ArmV4T {
    /// Implements the `MUL` and `MLA` instructions.
    pub fn multiply(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        let accumulate = instruction.check_bit(21);
        let should_set_condition = instruction.check_bit(20);
        let (reg_destination, reg_add) = arm::get_high_registers(instruction);
        let reg_1 = ((instruction >> 8) & 0xF) as usize;
        let reg_2 = (instruction & 0xF) as usize;

        // Check if the accumulate flag is set by casting it to u32, and then adding.
        // Doing this elides a branch (Sadly, compiler doesn't do it for us according to GodBolt :( )
        let result = cpu
            .read_reg(reg_1)
            .wrapping_mul(cpu.read_reg(reg_2))
            .wrapping_add(accumulate as u32 * cpu.read_reg(reg_add));
        cpu.write_reg(reg_destination, result, bus);

        if should_set_condition {
            cpu.registers.cpsr.set_sign(result.check_bit(31));
            cpu.registers.cpsr.set_zero(result == 0);
            // Carry flag set to meaningless value?
        }
    }

    pub fn multiply_long(cpu: &mut CPU, instruction: ArmInstruction, _bus: &mut Bus) {
        let unsigned = instruction.check_bit(22);
        let accumulate = instruction.check_bit(21);
        let should_set_condition = instruction.check_bit(20);
        let (reg_high, reg_low) = arm::get_high_registers(instruction);
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
            registers[reg_1] as i32 as i64 * registers[reg_2] as i32 as i64
        } else {
            registers[reg_1] as i32 as i64 * registers[reg_2] as i32 as i64
                + (((registers[reg_high] as i32 as i64) << 32) | registers[reg_low] as i32 as i64)
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
