use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmV4};
use crate::emulator::cpu::common::{common_behaviour, ShiftType};
use crate::emulator::cpu::registers::{Mode, PC_REG};
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;
use num_traits::FromPrimitive;

impl ArmV4 {
    // In the ArmInstruction opcode is bits 21..=24, and set_condition-code is bit 20
    #[grba_lut_generate::create_lut(u32, OPCODE=5..=8, SET_CONDITION_CODE=4)]
    #[inline(always)]
    pub fn data_processing_immediate<const OPCODE: u8, const SET_CONDITION_CODE: bool>(
        cpu: &mut CPU,
        instruction: ArmInstruction,
        bus: &mut Bus,
    ) {
        let opcode = OpCode::from_u8(OPCODE).unwrap();
        let r_d = instruction.get_bits(12, 15) as usize;

        let r_op1 = instruction.get_bits(16, 19) as usize;
        let op1_value = cpu.read_reg(r_op1);
        // We have to operate on an immediate value
        // Shift amount is 0 extended to 32 bits, then rotated right by `rotate amount * 2`
        let rotate = instruction.get_bits(8, 11) * 2;
        let imm = instruction.get_bits(0, 7);
        let op2_value = imm.rotate_right(rotate);
        // We don't use the barrel shifter `ShiftType::RotateRight` here because of this strange situation.
        // It seems that, if rotate == 0, then instead of using RotateRightExtended, it just doesn't change the carry flag.
        let carry = if rotate != 0 { op2_value.check_bit(31) } else { cpu.registers.cpsr.carry() };

        ArmV4::perform_data_operation(cpu, bus, opcode, op1_value, op2_value, r_d, SET_CONDITION_CODE, carry);
    }

    pub fn data_processing_register_immediate_shift(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        let opcode = OpCode::from_u32(instruction.get_bits(21, 24)).unwrap();
        let set_condition_code = instruction.check_bit(20);
        let r_d = instruction.get_bits(12, 15) as usize;

        let shift_type = ShiftType::from_u32(instruction.get_bits(5, 6)).unwrap();
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

        let opcode = OpCode::from_u32(instruction.get_bits(21, 24)).unwrap();
        let set_condition_code = instruction.check_bit(20);
        let r_d = instruction.get_bits(12, 15) as usize;

        let shift_type = ShiftType::from_u32(instruction.get_bits(5, 6)).unwrap();
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

    #[inline(always)]
    fn perform_data_operation(
        cpu: &mut CPU,
        bus: &mut Bus,
        opcode: OpCode,
        op1: u32,
        op2: u32,
        r_d: usize,
        set_flags: bool,
        barrel_shift_carry: bool,
    ) {
        let result = match opcode {
            OpCode::And => {
                let result = op1 & op2;
                if set_flags {
                    cpu.set_logical_flags(result, barrel_shift_carry);
                }

                result
            }
            OpCode::Eor => {
                let result = op1 ^ op2;
                if set_flags {
                    cpu.set_logical_flags(result, barrel_shift_carry);
                }

                result
            }
            OpCode::Sub => common_behaviour::sub(cpu, op1, op2, set_flags),
            OpCode::Rsb => common_behaviour::sub(cpu, op2, op1, set_flags),
            OpCode::Add => common_behaviour::add(cpu, op1, op2, set_flags),
            OpCode::Adc => common_behaviour::adc(cpu, op1, op2, set_flags),
            OpCode::Sbc => common_behaviour::sbc(cpu, op1, op2, set_flags),
            OpCode::Rsc => common_behaviour::sbc(cpu, op2, op1, set_flags),
            OpCode::Tst => {
                // Note, we're assuming that we can ignore the `set_flags` parameter here.
                cpu.set_logical_flags(op1 & op2, barrel_shift_carry);

                0
            }
            OpCode::Teq => {
                // Note, we're assuming that we can ignore the `set_flags` parameter here.
                cpu.set_logical_flags(op1 ^ op2, barrel_shift_carry);

                0
            }
            OpCode::Cmp => {
                // Normal sub, but we ignore the result
                let _ = common_behaviour::sub(cpu, op1, op2, true);

                0
            }
            OpCode::Cmn => {
                // Normal add, but we ignore the result
                let _ = common_behaviour::add(cpu, op1, op2, true);

                0
            }
            OpCode::Orr => {
                let result = op1 | op2;
                if set_flags {
                    cpu.set_logical_flags(result, barrel_shift_carry);
                }

                result
            }
            OpCode::Mov => {
                let result = op2;
                if set_flags {
                    cpu.set_logical_flags(result, barrel_shift_carry);
                }

                result
            }
            OpCode::Bic => {
                let result = op1 & !op2;
                if set_flags {
                    cpu.set_logical_flags(result, barrel_shift_carry);
                }

                result
            }
            OpCode::Mvn => {
                let result = !op2;
                if set_flags {
                    cpu.set_logical_flags(result, barrel_shift_carry);
                }

                result
            }
        };

        // If `r_d` is R15 and the S flag is set then the SPSR of the current mode is moved into the CPSR.
        // Primarily used for `MOVS` when returning from software interrupts.
        // Important to do this before the data operations due to force-alignment of PC on write
        //TODO: This r_d == 15 && set_flags check *may* have to be handled differently due to the fact that in arm.gba
        // we're currently failing the log diff on instruction 561 (cycle 1122), where the `CMP pc pc` sign flag is visible
        // in the other emu's log, but not in ours due to the CPSR being overwritten.
        // This test specifically: https://github.com/jsmolka/gba-tests/blob/a6447c5404c8fc2898ddc51f438271f832083b7e/arm/data_processing.asm#L498
        // Consider https://discord.com/channels/465585922579103744/465586361731121162/913580452395229186
        if r_d == 15 && set_flags {
            cpu.registers.write_cpsr(cpu.registers.spsr, bus);
        }

        match opcode {
            OpCode::Teq | OpCode::Tst | OpCode::Cmn | OpCode::Cmp => {}
            _ => cpu.write_reg(r_d, result, bus),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, num_derive::FromPrimitive)]
enum OpCode {
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
