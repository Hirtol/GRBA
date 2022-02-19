use crate::emulator::bus::Bus;
use crate::emulator::cpu::common::{common_behaviour, ShiftType};
use crate::emulator::cpu::thumb::{ThumbInstruction, ThumbV4};
use crate::emulator::cpu::CPU;
use crate::utils::{has_sign_overflowed, BitOps};

#[derive(Debug)]
enum ShiftOpcode {
    Lsl = 0x0,
    Lsr = 0x1,
    Asr = 0x2,
}

impl From<u16> for ShiftOpcode {
    fn from(val: u16) -> Self {
        match val {
            0x0 => ShiftOpcode::Lsl,
            0x1 => ShiftOpcode::Lsr,
            0x2 => ShiftOpcode::Asr,
            _ => panic!("Invalid opcode"),
        }
    }
}

impl From<ShiftOpcode> for ShiftType {
    fn from(op: ShiftOpcode) -> Self {
        match op {
            ShiftOpcode::Lsl => ShiftType::LogicalLeft,
            ShiftOpcode::Lsr => ShiftType::LogicalRight,
            ShiftOpcode::Asr => ShiftType::ArithRight,
        }
    }
}

impl ThumbV4 {
    pub fn move_shifted_reg(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let opcode: ShiftOpcode = instruction.get_bits(11, 12).into();
        let offset = instruction.get_bits(6, 10) as u8;
        let r_d = instruction.get_bits(0, 2) as usize;
        let r_s = instruction.get_bits(3, 5) as usize;

        let shift_type = ShiftType::from(opcode);

        let (value, carry) = shift_type.perform_shift(cpu.read_reg(r_s), offset, cpu.registers.cpsr.carry());

        cpu.write_reg(r_d, value, bus);
        cpu.set_logical_flags(value, carry);
    }

    //15 14 13 12 11 10 9
    // 0  0  0  1  1  I OP
    pub fn add_subtract(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let is_immediate = instruction.check_bit(10);
        let is_sub = instruction.check_bit(9);
        let r_n_or_immediate = instruction.get_bits(6, 8) as usize;
        let r_s = instruction.get_bits(3, 5) as usize;
        let r_d = instruction.get_bits(0, 2) as usize;

        let to_add = if is_immediate { r_n_or_immediate as u32 } else { cpu.read_reg(r_n_or_immediate) };
        let s_contents = cpu.read_reg(r_s);
        let (to_write, carry) =
            if is_sub { s_contents.overflowing_sub(to_add) } else { s_contents.overflowing_add(to_add) };

        cpu.write_reg(r_d, to_write, bus);

        cpu.set_arithmetic_flags(to_write, carry, has_sign_overflowed(s_contents, to_add, to_write));
    }

    //15 14 13 12 11
    // 0  0  1  O  P
    pub fn move_compare_add_subtract(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        enum Opcode {
            Mov = 0b00,
            Cmp = 0b01,
            Add = 0b10,
            Sub = 0b11,
        }

        let opcode = instruction.get_bits(11, 12);
        let r_d = instruction.get_bits(8, 10) as usize;
        let offset = instruction.get_bits(0, 7) as u32;
        let current_value = cpu.read_reg(r_d);

        match opcode {
            0b00 => {
                cpu.write_reg(r_d, offset, bus);
            }
            0b01 => {
                // Same as sub, only write flags however
                let _ = common_behaviour::sub(cpu, current_value, offset, true);
            }
            0b10 => {
                let result = common_behaviour::add(cpu, current_value, offset, true);
                cpu.write_reg(r_d, result, bus);
            }
            0b11 => {
                let result = common_behaviour::sub(cpu, current_value, offset, true);
                cpu.write_reg(r_d, result, bus);
            }
            _ => unreachable!(),
        }
    }
}
