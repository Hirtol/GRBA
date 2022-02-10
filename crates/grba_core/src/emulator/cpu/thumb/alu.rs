use crate::emulator::bus::Bus;
use crate::emulator::cpu::common::ShiftType;
use crate::emulator::cpu::thumb::{ThumbInstruction, ThumbV4};
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;

#[derive(num_derive::FromPrimitive, Debug)]
enum Opcode {
    Lsl = 0x0,
    Lsr = 0x1,
    Asr = 0x2,
}

impl From<u16> for Opcode {
    fn from(val: u16) -> Self {
        match val {
            0x0 => Opcode::Lsl,
            0x1 => Opcode::Lsr,
            0x2 => Opcode::Asr,
            _ => panic!("Invalid opcode"),
        }
    }
}

impl From<Opcode> for ShiftType {
    fn from(op: Opcode) -> Self {
        match op {
            Opcode::Lsl => ShiftType::LogicalLeft,
            Opcode::Lsr => ShiftType::LogicalRight,
            Opcode::Asr => ShiftType::ArithRight,
        }
    }
}

impl ThumbV4 {
    pub fn move_shifted_reg(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let opcode: Opcode = instruction.get_bits(11, 12).into();
        let offset = instruction.get_bits(6, 10) as u8;
        let r_d = instruction.get_bits(0, 2) as usize;
        let r_s = instruction.get_bits(3, 5) as usize;

        let shift_type = ShiftType::from(opcode);

        let (value, carry) = shift_type.perform_shift(cpu.read_reg(r_s), offset, cpu.registers.cpsr.carry());

        cpu.write_reg(r_d, value, bus);
        cpu.set_logical_flags(value, carry);
    }
}
