use crate::bus::Bus;
use crate::cpu::arm::{ArmInstruction, ArmV4T};
use crate::cpu::registers::{State, LINK_REG, PC_REG};
use crate::cpu::CPU;
use crate::utils::BitOps;

impl ArmV4T {
    pub fn branch_and_exchange(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        let r_n = instruction.get_bits(0, 3) as usize;
        let reg_contents = cpu.read_reg(r_n);

        let to_thumb = reg_contents.check_bit(0);

        // If there is a new state, switch to it
        cpu.switch_state(if to_thumb { State::Thumb } else { State::Arm }, bus);

        // Write new PC value, definitely flushes the pipeline
        cpu.write_reg(PC_REG, reg_contents, bus);
    }

    pub fn branch_and_link(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        let is_link = instruction.check_bit(24);
        let offset = (instruction.get_bits(0, 23) << 2) as i32;
        let pc = cpu.read_reg(PC_REG);

        if is_link {
            // Write the old PC value to the link register.
            // Subtract an  adjustment to account for our pre-fetching (where we're 2 instructions ahead).
            let prefetch_adjust = if cpu.state() == State::Thumb { 2 } else { 4 };
            cpu.write_reg(LINK_REG, pc.wrapping_sub(prefetch_adjust), bus);
        }

        cpu.write_reg(PC_REG, pc.wrapping_add(offset as u32), bus);
    }
}
