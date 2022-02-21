use crate::emulator::bus::Bus;
use crate::emulator::cpu::registers::{LINK_REG, PC_REG, SP_REG};
use crate::emulator::cpu::thumb::{ThumbInstruction, ThumbV4};
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;

impl ThumbV4 {
    pub fn push_pop_registers(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let is_load = instruction.check_bit(11);
        let store_lr_load_pc = instruction.check_bit(8);

        let register_list = instruction.get_bits(0, 7) as u8;

        if is_load {
            if store_lr_load_pc {
                let sp = cpu.read_reg(SP_REG);
                cpu.write_reg(PC_REG, bus.read_32(sp), bus);

                cpu.write_reg(SP_REG, sp.wrapping_add(4), bus);
            }

            for i in 0..8 {
                if register_list.check_bit(i) {
                    let sp = cpu.read_reg(SP_REG);
                    cpu.write_reg(i as usize, bus.read_32(sp), bus);

                    cpu.write_reg(SP_REG, sp.wrapping_add(4), bus);
                }
            }
        } else {
            if store_lr_load_pc {
                let sp = cpu.read_reg(SP_REG);
                bus.write_32(sp, cpu.read_reg(LINK_REG));
                cpu.write_reg(SP_REG, sp.wrapping_sub(4), bus);
            }

            for i in 0..8 {
                if register_list.check_bit(i) {
                    let reg_value = cpu.read_reg(i as usize);
                    let sp = cpu.read_reg(SP_REG);
                    bus.write_32(sp, reg_value);

                    cpu.write_reg(SP_REG, sp.wrapping_sub(4), bus);
                }
            }
        }
    }
}
