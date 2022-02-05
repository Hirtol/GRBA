use crate::bus::Bus;
use crate::cpu::arm::{ArmInstruction, ArmV4T, ShiftType};
use crate::cpu::registers::Mode;
use crate::cpu::CPU;
use crate::utils::{has_sign_overflowed, BitOps};
use num_traits::FromPrimitive;

impl ArmV4T {
    pub fn data_processing(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        crate::cpu_log!("Executing instruction: Data Processing");
        let is_immediate = instruction.check_bit(25);
        let opcode = DataOperation::from_u32(instruction.get_bits(21, 24)).unwrap();
        let set_condition_code = instruction.check_bit(20);
        let r_d = instruction.get_bits(12, 15) as usize;
        // If `r_d` is R15 and the S flag is set then the SPSR of the current mode is moved into the CPSR.
        // If the current mode is user mode we do nothing. (TODO: Check how necessary the user mode check is, spec technically asks for it)
        if r_d == 15 && set_condition_code && cpu.registers.cpsr.mode() != Mode::User {
            cpu.registers.cpsr = cpu.registers.spsr;
        }

        if is_immediate {
            let r_op1 = instruction.get_bits(16, 19) as usize;
            let op1_value = cpu.read_reg(r_op1);
            // We have to operate on an immediate value
            // Shift amount is 0 extended to 32 bits, then rotated right by `rotate amount * 2`
            let rotate = instruction.get_bits(8, 11) * 2;
            let imm = instruction.get_bits(0, 7) as u32;
            let op2_value = imm.rotate_right(rotate);

            ArmV4T::perform_data_operation(cpu, bus, opcode, op1_value, op2_value, r_d, set_condition_code);
        } else {
            // We are working with a register
            let shift_type = ShiftType::from_u32(instruction.get_bits(5, 6)).unwrap();
            // r_m
            let r_op2 = instruction.get_bits(0, 3) as usize;

            // Check the shift type
            let should_shift_register = instruction.check_bit(4);

            let op2_value = if should_shift_register {
                // Register Shift, we'll need to increment PC by 4 for the duration of this function, refer to section 4.5.5 of the instruction manual.
                cpu.registers.general_purpose[15] += 4;
                let shift_register = instruction.get_bits(8, 11) as usize;
                // Only the lower byte matters, can just directly cast to a u8
                let shift_amount = cpu.read_reg(shift_register) as u8;

                let (op2_value, carry) = if shift_amount == 0 {
                    (cpu.read_reg(r_op2), cpu.registers.cpsr.carry())
                } else {
                    shift_type.perform_shift(cpu.read_reg(r_op2), shift_amount, cpu.registers.cpsr.carry())
                };

                //TODO: This might cause issues if we have an arithmetic instruction which doesn't set condition codes,
                // as the previous carry value would be overwritten.
                cpu.registers.cpsr.set_carry(carry);
                op2_value
            } else {
                // Immediate Shift
                let shift_amount = instruction.get_bits(7, 11) as u8;
                let (op2_value, carry) =
                    shift_type.perform_shift(cpu.read_reg(r_op2), shift_amount, cpu.registers.cpsr.carry());

                //TODO: This might cause issues if we have an arithmetic instruction which doesn't set condition codes,
                // as the previous carry value would be overwritten.
                cpu.registers.cpsr.set_carry(carry);

                op2_value
            };

            let r_op1 = instruction.get_bits(16, 19) as usize;
            let op1_value = cpu.read_reg(r_op1);

            ArmV4T::perform_data_operation(cpu, bus, opcode, op1_value, op2_value, r_d, set_condition_code);
        }
    }

    fn perform_data_operation(
        cpu: &mut CPU,
        bus: &mut Bus,
        opcode: DataOperation,
        op1: u32,
        op2: u32,
        r_d: usize,
        set_flags: bool,
    ) {
        crate::cpu_log!("Executing opcode: {:?}", opcode);
        match opcode {
            DataOperation::And => {
                let result = op1 & op2;
                cpu.write_reg(r_d, result, bus);
                if set_flags {
                    cpu.set_logical_flags(result);
                }
            }
            DataOperation::Eor => {
                let result = op1 ^ op2;
                cpu.write_reg(r_d, result, bus);
                if set_flags {
                    cpu.set_logical_flags(result);
                }
            }
            DataOperation::Sub => {
                ArmV4T::arm_sub(cpu, bus, r_d, op1, op2, set_flags);
            }
            DataOperation::Rsb => ArmV4T::arm_sub(cpu, bus, r_d, op2, op1, set_flags),
            DataOperation::Add => {
                let (result, carry) = op1.overflowing_add(op2);
                cpu.write_reg(r_d, result, bus);
                if set_flags {
                    cpu.set_arithmetic_flags(result, carry, has_sign_overflowed(op1, op2, result));
                }
            }
            DataOperation::Adc => {
                let full_result = op1 as u64 + op2 as u64 + cpu.registers.cpsr.carry() as u64;
                let result = full_result as u32;
                cpu.write_reg(r_d, result, bus);
                if set_flags {
                    cpu.set_arithmetic_flags(result, full_result.check_bit(32), has_sign_overflowed(op1, op2, result));
                }
            }
            DataOperation::Sbc => {
                ArmV4T::arm_sbc(cpu, bus, r_d, op1, op2, set_flags);
            }
            DataOperation::Rsc => ArmV4T::arm_sbc(cpu, bus, r_d, op2, op1, set_flags),
            DataOperation::Tst => {
                let result = op1 & op2;
                // Note, we're assuming that we can ignore the `set_flags` parameter here.
                cpu.set_logical_flags(result);
            }
            DataOperation::Teq => {
                let result = op1 ^ op2;
                // Note, we're assuming that we can ignore the `set_flags` parameter here.
                cpu.set_logical_flags(result);
            }
            DataOperation::Cmp => {
                let (result, carry) = op1.overflowing_sub(op2);
                // Note, we're assuming that we can ignore the `set_flags` parameter here.
                cpu.set_arithmetic_flags(result, carry, has_sign_overflowed(op1, op2, result));
            }
            DataOperation::Cmn => {
                let (result, carry) = op1.overflowing_add(op2);
                // Note, we're assuming that we can ignore the `set_flags` parameter here.
                cpu.set_arithmetic_flags(result, carry, has_sign_overflowed(op1, op2, result));
            }
            DataOperation::Orr => {
                let result = op1 | op2;
                cpu.write_reg(r_d, result, bus);
                if set_flags {
                    cpu.set_logical_flags(result);
                }
            }
            DataOperation::Mov => {
                let result = op2;
                cpu.write_reg(r_d, result, bus);
                if set_flags {
                    cpu.set_logical_flags(result);
                }
            }
            DataOperation::Bic => {
                let result = op1 & !op2;
                cpu.write_reg(r_d, result, bus);
                if set_flags {
                    cpu.set_logical_flags(result);
                }
            }
            DataOperation::Mvn => {
                let result = !op2;
                cpu.write_reg(r_d, result, bus);
                if set_flags {
                    cpu.set_logical_flags(result);
                }
            }
        };
    }

    fn set_logical_flags(cpu: &mut CPU, value: u32) {
        cpu.registers.cpsr.set_zero(value == 0);
        cpu.registers.cpsr.set_sign(value.check_bit(31));
    }

    fn set_arithmetic_flags(cpu: &mut CPU, value: u32, carry: bool, overflow: bool) {
        cpu.set_logical_flags(value);
        cpu.registers.cpsr.set_carry(carry);
        cpu.registers.cpsr.set_overflow(overflow);
    }

    fn arm_sub(cpu: &mut CPU, bus: &mut Bus, r_d: usize, op1: u32, op2: u32, set_flags: bool) {
        let (result, carry) = op1.overflowing_sub(op2);
        cpu.write_reg(r_d, result, bus);
        if set_flags {
            cpu.set_arithmetic_flags(result, carry, has_sign_overflowed(op1, op2, result));
        }
    }

    fn arm_sbc(cpu: &mut CPU, bus: &mut Bus, r_d: usize, op1: u32, op2: u32, set_flags: bool) {
        let to_subtract = op2 as u64 + 1 - cpu.registers.cpsr.carry() as u64;
        let (full_result, carry) = (op1 as u64).overflowing_sub(to_subtract);
        let result = full_result as u32;

        cpu.write_reg(r_d, result, bus);
        if set_flags {
            cpu.set_arithmetic_flags(result, carry, has_sign_overflowed(op1, op2, result));
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, num_derive::FromPrimitive)]
enum DataOperation {
    And = 0b0000,
    Eor = 0b0001,
    Sub = 0b0010,
    Rsb = 0b0011,
    Add = 0b0100,
    Adc = 0b0101,
    Sbc = 0b0110,
    Rsc = 0b0111,
    Tst = 0b1000,
    Teq = 0b1001,
    Cmp = 0b1010,
    Cmn = 0b1011,
    Orr = 0b1100,
    Mov = 0b1101,
    Bic = 0b1110,
    Mvn = 0b1111,
}

impl CPU {
    fn set_logical_flags(&mut self, value: u32) {
        self.registers.cpsr.set_zero(value == 0);
        self.registers.cpsr.set_sign(value.check_bit(31));
    }

    fn set_arithmetic_flags(&mut self, value: u32, carry: bool, overflow: bool) {
        self.set_logical_flags(value);
        self.registers.cpsr.set_carry(carry);
        self.registers.cpsr.set_overflow(overflow);
    }
}
