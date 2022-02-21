use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmV4};
use crate::emulator::cpu::common::{common_behaviour, ShiftType};
use crate::emulator::cpu::registers::{Mode, PC_REG};
use crate::emulator::cpu::CPU;
use crate::utils::{has_sign_overflowed, BitOps};
use num_traits::FromPrimitive;

impl ArmV4 {
    pub fn data_processing_immediate(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        let opcode = DataOperation::from_u32(instruction.get_bits(21, 24)).unwrap();
        let set_condition_code = instruction.check_bit(20);
        let r_d = instruction.get_bits(12, 15) as usize;
        // If `r_d` is R15 and the S flag is set then the SPSR of the current mode is moved into the CPSR.
        // If the current mode is user mode we do nothing. (TODO: Check how necessary the user mode check is, spec technically asks for it)
        if r_d == 15 && set_condition_code && cpu.registers.cpsr.mode() != Mode::User {
            cpu.registers.cpsr = cpu.registers.spsr;
        }

        let r_op1 = instruction.get_bits(16, 19) as usize;
        let op1_value = cpu.read_reg(r_op1);
        // We have to operate on an immediate value
        // Shift amount is 0 extended to 32 bits, then rotated right by `rotate amount * 2`
        let rotate = instruction.get_bits(8, 11) * 2;
        let imm = instruction.get_bits(0, 7) as u32;
        let op2_value = imm.rotate_right(rotate);
        // We don't use the barrel shifter `ShiftType::RotateRight` here because of this strange situation.
        // It seems that, if rotate == 0, then instead of using RotateRightExtended, it just doesn't change the carry flag.
        let carry = if rotate != 0 { op2_value.check_bit(31) } else { cpu.registers.cpsr.carry() };

        ArmV4::perform_data_operation(cpu, bus, opcode, op1_value, op2_value, r_d, set_condition_code, carry);
    }

    pub fn data_processing_register_immediate_shift(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        let opcode = DataOperation::from_u32(instruction.get_bits(21, 24)).unwrap();
        let set_condition_code = instruction.check_bit(20);
        let r_d = instruction.get_bits(12, 15) as usize;
        // If `r_d` is R15 and the S flag is set then the SPSR of the current mode is moved into the CPSR.
        // If the current mode is user mode we do nothing. (TODO: Check how necessary the user mode check is, spec technically asks for it)
        if r_d == 15 && set_condition_code && cpu.registers.cpsr.mode() != Mode::User {
            cpu.registers.cpsr = cpu.registers.spsr;
        }

        let shift_type = ShiftType::from_u32(instruction.get_bits(5, 6)).unwrap();
        // r_m
        let r_op2 = instruction.get_bits(0, 3) as usize;

        let (op2_value, carry) = {
            // Immediate Shift
            let shift_amount = instruction.get_bits(7, 11) as u8;

            shift_type.perform_shift(cpu.read_reg(r_op2), shift_amount, cpu.registers.cpsr.carry())
        };

        let r_op1 = instruction.get_bits(16, 19) as usize;
        let op1_value = cpu.read_reg(r_op1);

        ArmV4::perform_data_operation(cpu, bus, opcode, op1_value, op2_value, r_d, set_condition_code, carry);
    }

    pub fn data_processing_register_register_shift(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        //we'll need to increment PC by 4 for the duration of this function, refer to section 4.5.5 of the instruction manual.
        cpu.registers.general_purpose[PC_REG] += 4;

        let opcode = DataOperation::from_u32(instruction.get_bits(21, 24)).unwrap();
        let set_condition_code = instruction.check_bit(20);
        let r_d = instruction.get_bits(12, 15) as usize;
        // If `r_d` is R15 and the S flag is set then the SPSR of the current mode is moved into the CPSR.
        // If the current mode is user mode we do nothing. (TODO: Check how necessary the user mode check is, spec technically asks for it)
        if r_d == 15 && set_condition_code && cpu.registers.cpsr.mode() != Mode::User {
            cpu.registers.cpsr = cpu.registers.spsr;
        }

        let shift_type = ShiftType::from_u32(instruction.get_bits(5, 6)).unwrap();
        // r_m
        let r_op2 = instruction.get_bits(0, 3) as usize;

        let (op2_value, carry) = {
            // Register Shift
            let shift_register = instruction.get_bits(8, 11) as usize;
            // Only the lower byte matters, can just directly cast to a u8
            let shift_amount = cpu.read_reg(shift_register) as u8;

            if shift_amount == 0 {
                (cpu.read_reg(r_op2), cpu.registers.cpsr.carry())
            } else {
                shift_type.perform_shift(cpu.read_reg(r_op2), shift_amount, cpu.registers.cpsr.carry())
            }
        };

        let r_op1 = instruction.get_bits(16, 19) as usize;
        let op1_value = cpu.read_reg(r_op1);

        ArmV4::perform_data_operation(cpu, bus, opcode, op1_value, op2_value, r_d, set_condition_code, carry);

        // Undo our increment from before
        cpu.registers.general_purpose[PC_REG] -= 4;
    }

    fn perform_data_operation(
        cpu: &mut CPU,
        bus: &mut Bus,
        opcode: DataOperation,
        op1: u32,
        op2: u32,
        r_d: usize,
        set_flags: bool,
        barrel_shift_carry: bool,
    ) {
        crate::cpu_log!("Executing opcode: {:?}", opcode);
        match opcode {
            DataOperation::And => {
                let result = op1 & op2;
                cpu.write_reg(r_d, result, bus);
                if set_flags {
                    cpu.set_logical_flags(result, barrel_shift_carry);
                }
            }
            DataOperation::Eor => {
                let result = op1 ^ op2;
                cpu.write_reg(r_d, result, bus);
                if set_flags {
                    cpu.set_logical_flags(result, barrel_shift_carry);
                }
            }
            DataOperation::Sub => {
                let result = common_behaviour::sub(cpu, op1, op2, set_flags);

                cpu.write_reg(r_d, result, bus);
            }
            DataOperation::Rsb => {
                let result = common_behaviour::sub(cpu, op2, op1, set_flags);

                cpu.write_reg(r_d, result, bus);
            }
            DataOperation::Add => {
                let result = common_behaviour::add(cpu, op1, op2, set_flags);

                cpu.write_reg(r_d, result, bus);
            }
            DataOperation::Adc => {
                let result = common_behaviour::adc(cpu, op1, op2, set_flags);

                cpu.write_reg(r_d, result, bus);
            }
            DataOperation::Sbc => {
                let result = common_behaviour::sbc(cpu, op1, op2, set_flags);

                cpu.write_reg(r_d, result, bus);
            }
            DataOperation::Rsc => {
                let result = common_behaviour::sbc(cpu, op2, op1, set_flags);

                cpu.write_reg(r_d, result, bus);
            }
            DataOperation::Tst => {
                let result = op1 & op2;
                // Note, we're assuming that we can ignore the `set_flags` parameter here.
                cpu.set_logical_flags(result, barrel_shift_carry);
            }
            DataOperation::Teq => {
                let result = op1 ^ op2;
                // Note, we're assuming that we can ignore the `set_flags` parameter here.
                cpu.set_logical_flags(result, barrel_shift_carry);
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
                    cpu.set_logical_flags(result, barrel_shift_carry);
                }
            }
            DataOperation::Mov => {
                let result = op2;
                cpu.write_reg(r_d, result, bus);
                if set_flags {
                    cpu.set_logical_flags(result, barrel_shift_carry);
                }
            }
            DataOperation::Bic => {
                let result = op1 & !op2;
                cpu.write_reg(r_d, result, bus);
                if set_flags {
                    cpu.set_logical_flags(result, barrel_shift_carry);
                }
            }
            DataOperation::Mvn => {
                let result = !op2;
                cpu.write_reg(r_d, result, bus);
                if set_flags {
                    cpu.set_logical_flags(result, barrel_shift_carry);
                }
            }
        };
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
