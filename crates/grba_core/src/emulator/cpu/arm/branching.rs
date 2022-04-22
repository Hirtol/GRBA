use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmV4};
use crate::emulator::cpu::common::common_behaviour;
use crate::emulator::cpu::registers::{LINK_REG, PC_REG};
use crate::emulator::cpu::CPU;
use crate::utils::{sign_extend32, BitOps};

impl ArmV4 {
    pub fn branch_and_exchange(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        let r_n = instruction.get_bits(0, 3) as usize;
        let reg_contents = cpu.read_reg(r_n);

        common_behaviour::branch_and_exchange(cpu, reg_contents, bus);
    }

    // `is_link` is actually bit 24 of the instruction, but for our LUT lookup we use bits 20..=27 and bits 4..=7
    #[grba_lut_generate::create_lut(u32, IS_LINK = 8)]
    pub fn branch_and_link<const IS_LINK: bool>(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        let offset = sign_extend32(instruction.get_bits(0, 23), 24) << 2;
        let pc = cpu.read_reg(PC_REG);

        if IS_LINK {
            // Write the old PC value to the link register.
            // Subtract an  adjustment to account for our pre-fetching (where we're 2 instructions ahead).
            cpu.write_reg(LINK_REG, pc.wrapping_sub(4), bus);
        }

        cpu.write_reg(PC_REG, pc.wrapping_add(offset as u32), bus);
    }
}
