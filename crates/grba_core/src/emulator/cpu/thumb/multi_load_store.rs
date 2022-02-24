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

    pub fn multiple_load_store(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let is_load = instruction.check_bit(11);
        let r_base = instruction.get_bits(8, 10) as usize;

        let register_list = instruction.get_bits(0, 7) as u8;
        let mut base_address = cpu.read_reg(r_base);

        if is_load {
            for i in 0..8 {
                if register_list.check_bit(i) {
                    cpu.write_reg(i as usize, bus.read_32(base_address), bus);
                    //TODO: Should this be adding or subtracting? (Store is definitely incrementing)
                    base_address = base_address.wrapping_add(4);
                }
            }
        } else {
            for i in 0..8 {
                if register_list.check_bit(i) {
                    let reg_value = cpu.read_reg(i as usize);
                    bus.write_32(base_address, reg_value);

                    base_address = base_address.wrapping_add(4);
                }
            }
        }

        cpu.write_reg(r_base, base_address, bus);
    }
}