use crate::emulator::bus::Bus;
use crate::emulator::cpu::common::common_behaviour;
use crate::emulator::cpu::registers::{Mode, State, LINK_REG, PC_REG};
use crate::emulator::cpu::thumb::{ThumbInstruction, ThumbV4};
use crate::emulator::cpu::{Exception, CPU};
use crate::utils::{sign_extend32, BitOps};
use num_traits::FromPrimitive;

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

    pub fn conditional_branch(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let condition = instruction.get_bits(8, 11);
        let offset = sign_extend32((instruction.get_bits(0, 7) << 1) as u32, 9);

        match condition {
            // Software interrupt
            0b1111 => {
                cpu.raise_exception(bus, Exception::SoftwareInterrupt);
            }
            _ => {
                if common_behaviour::check_condition(&cpu.registers.cpsr, condition as u8) {
                    let pc = cpu.read_reg(PC_REG);
                    cpu.write_reg(PC_REG, pc.wrapping_add(offset as u32), bus);
                }
            }
        };
    }

    pub fn unconditional_branch(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let offset = sign_extend32((instruction.get_bits(0, 10) << 1) as u32, 12);
        let pc = cpu.read_reg(PC_REG);
        cpu.write_reg(PC_REG, pc.wrapping_add(offset as u32), bus);
    }

    pub fn long_branch_with_link_high(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let offset = sign_extend32((instruction.get_bits(0, 10) as u32) << 12, 23);
        let pc = cpu.read_reg(PC_REG);

        cpu.write_reg(LINK_REG, pc.wrapping_add(offset as u32), bus);
    }

    pub fn long_branch_with_link_low(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let offset = sign_extend32((instruction.get_bits(0, 10) as u32) << 1, 12);
        let lr = cpu.read_reg(LINK_REG);
        let final_value = lr.wrapping_add(offset as u32);
        let next_instruction_address = cpu.read_reg(PC_REG).wrapping_sub(2);

        cpu.write_reg(PC_REG, final_value, bus);
        // The link register should contain the next instruction's (at this instruction, aka, before the jump) address
        // Bitwise or with 1
        cpu.write_reg(LINK_REG, next_instruction_address | 1, bus);
    }
}
