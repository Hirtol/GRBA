use crate::emulator::bus::Bus;
use crate::emulator::cpu::registers::{LINK_REG, PC_REG, SP_REG};
use crate::emulator::cpu::thumb::{ThumbInstruction, ThumbV4};
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;

impl ThumbV4 {
    pub fn push_pop_registers(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let is_pop = instruction.check_bit(11);
        let store_lr_load_pc = instruction.check_bit(8);

        let register_list = instruction.get_bits(0, 7) as u8;

        // TODO: Consider handling invalid R-Lists in multiple load/store instructions

        if is_pop {
            let mut sp = cpu.read_reg(SP_REG);

            for i in 0..8 {
                if register_list.check_bit(i) {
                    sp = sp.wrapping_add(4);

                    cpu.write_reg(i as usize, bus.read_32(sp & 0xFFFF_FFFC, cpu), bus);
                }
            }

            if store_lr_load_pc {
                sp = sp.wrapping_add(4);
                cpu.write_reg(PC_REG, bus.read_32(sp & 0xFFFF_FFFC, cpu), bus);
            }

            cpu.write_reg(SP_REG, sp, bus);
        } else {
            if store_lr_load_pc {
                let sp = cpu.read_reg(SP_REG);
                bus.write_32(sp & 0xFFFF_FFFC, cpu.read_reg(LINK_REG));
                cpu.write_reg(SP_REG, sp.wrapping_sub(4), bus);
            }

            for i in (0..8).rev() {
                if register_list.check_bit(i) {
                    let reg_value = cpu.read_reg(i as usize);
                    let sp = cpu.read_reg(SP_REG);
                    bus.write_32(sp & 0xFFFF_FFFC, reg_value);

                    cpu.write_reg(SP_REG, sp.wrapping_sub(4), bus);
                }
            }
        }
    }

    pub fn multiple_store(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        // For the duration of this instruction PC will be 6 ahead instead of just 4.
        cpu.registers.general_purpose[PC_REG] += 2;
        Self::multiple_load_store(cpu, instruction, bus, false);
        cpu.registers.general_purpose[PC_REG] -= 2;
    }

    pub fn multiple_load(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        Self::multiple_load_store(cpu, instruction, bus, true);
    }

    #[inline(always)]
    fn multiple_load_store(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus, is_load: bool) {
        let r_base = instruction.get_bits(8, 10) as usize;

        let register_list = instruction.get_bits(0, 7) as u8;
        let mut base_address = cpu.read_reg(r_base);
        let mut has_writeback = true;

        // Handle edge case of empty register list.
        if register_list == 0 {
            // Empty register list is interpreted as PC_REG being transferred.
            if is_load {
                cpu.write_reg(PC_REG, bus.read_32(base_address, cpu), bus);
            } else {
                let reg_value = cpu.read_reg(PC_REG);
                bus.write_32(base_address, reg_value);
            }

            // When the register list is empty we add 0x40 to the base address.
            cpu.write_reg(r_base, base_address + 0x40, bus);

            return;
        }

        // Edge cases for where the base register is included in the store/load register list
        if register_list.check_bit(r_base as u8) {
            if is_load {
                // In ARMv4/v5 writeback gets disabled
                has_writeback = false
            } else {
                // In ARMv4 we store the old base if the register is *first* in the list, otherwise store new base
                // In ARMv5 we always store the new base
                let registers = register_list << (7 - r_base);

                // Is not first in the list, then we store the new base
                if registers.count_ones() != 1 {
                    // Store the NEW base ahead of time
                    let final_address = base_address.wrapping_add(4 * register_list.count_ones());
                    cpu.write_reg(r_base, final_address, bus);
                }
            }
        }

        if is_load {
            for i in 0..8 {
                if register_list.check_bit(i) {
                    cpu.write_reg(i as usize, bus.read_32(base_address, cpu), bus);

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

        if has_writeback {
            cpu.write_reg(r_base, base_address, bus);
        }
    }
}
