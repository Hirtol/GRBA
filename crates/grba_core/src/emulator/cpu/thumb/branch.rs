use crate::emulator::bus::Bus;
use crate::emulator::cpu::common::common_behaviour;
use crate::emulator::cpu::registers::PC_REG;
use crate::emulator::cpu::thumb::{ThumbInstruction, ThumbV4};
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;

impl ThumbV4 {
    pub fn hi_reg_op_branch_exchange(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        enum Opcode {
            Add = 0b00,
            Cmp = 0b01,
            Mov = 0b10,
            Bx = 0b11,
        }

        let opcode = instruction.get_bits(8, 9);
        // Note that !h1 && !h2 for any opcode is undefined behaviour, still needs to be figured out, we'll assume normal operation!
        let h1 = instruction.check_bit(7);
        let h2 = instruction.check_bit(6);
        // We do a branchless add here for getting the high (8-15) registers if the flags are set.
        let r_d = instruction.get_bits(0, 2) as usize + (h1 as usize * 8);
        let r_s = instruction.get_bits(3, 5) as usize + (h2 as usize * 8);

        let op1 = cpu.read_reg(r_d);
        let op2 = cpu.read_reg(r_s);

        match opcode {
            0b00 => {
                // Flags are never set
                let result = common_behaviour::add(cpu, op1, op2, false);

                cpu.write_reg(r_d, result, bus);
            }
            0b01 => {
                let _ = common_behaviour::sub(cpu, op1, op2, true);
            }
            0b10 => {
                cpu.write_reg(r_d, op2, bus);
            }
            0b11 => {
                common_behaviour::branch_and_exchange(cpu, op2, bus);
            }
            _ => unreachable!(),
        }
    }

    pub fn pc_relative_load(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let r_d = instruction.get_bits(8, 10) as usize;

        let imm_value = instruction.get_bits(0, 7) << 2;
        // PC value must always be word aligned for this addition
        let pc_value = cpu.registers.pc() & 0xFFFFFFFC;

        cpu.write_reg(r_d, pc_value.wrapping_add(imm_value as u32), bus);
    }
}
