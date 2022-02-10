use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmV4T, ShiftType};
use crate::emulator::cpu::registers::{Mode, PC_REG};
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;

impl ArmV4T {
    pub fn block_data_transfer_store(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        crate::cpu_log!("Executing instruction: Block Data Store");
        // For the duration of this instruction PC will be 12 ahead instead of just 8.
        cpu.registers.general_purpose[PC_REG] += 4;
        Self::block_data_transfer(cpu, instruction, bus, false);
        cpu.registers.general_purpose[PC_REG] -= 4;
    }

    pub fn block_data_transfer_load(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        crate::cpu_log!("Executing instruction: Block Data Load");
        Self::block_data_transfer(cpu, instruction, bus, true);
    }

    #[inline(always)]
    fn block_data_transfer(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus, is_load: bool) {
        let psr_or_user = instruction.check_bit(22);

        if psr_or_user {
            Self::block_data_transfer_with_s_bit(cpu, instruction, bus, is_load);
        } else {
            Self::block_data_transfer_without_s_bit(cpu, instruction, bus, is_load);
        }
    }

    #[inline]
    fn block_data_transfer_without_s_bit(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus, is_load: bool) {
        let is_preindexed = instruction.check_bit(24);
        let is_up = instruction.check_bit(23);
        let has_writeback = instruction.check_bit(21);

        let register_list = instruction.get_bits(0, 15) as u16;
        let register_count = register_list.count_ones();
        let reg_base = instruction.get_bits(16, 19) as usize;

        let mut address = cpu.read_reg(reg_base);
        let final_address;

        if is_up {
            final_address = address.wrapping_add(4 * register_count);
        } else {
            final_address = address.wrapping_sub(4 * register_count);
            if is_preindexed {
                // Pre increment will need to  be one lower
                address = final_address.wrapping_sub(4);
            } else {
                // Post decrement starts one higher than pre decrement, but both end at same address
                address = final_address.wrapping_add(4);
            }
        }

        // Inefficient iteration for now, TODO: Optimise.
        for i in 0..16 {
            if register_list.check_bit(i) {
                if is_preindexed {
                    address = address.wrapping_add(4)
                }

                let reg_dest = i as usize;
                if is_load {
                    let value = bus.read_32(address);
                    cpu.write_reg(reg_dest, value, bus);
                } else {
                    let value = cpu.read_reg(reg_dest);
                    bus.write_32(address, value);
                }

                if !is_preindexed {
                    address = address.wrapping_add(4)
                }
            }
        }

        if has_writeback {
            cpu.write_reg(reg_base, final_address, bus);
        }
    }

    #[inline]
    fn block_data_transfer_with_s_bit(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus, is_load: bool) {
        let is_preindexed = instruction.check_bit(24);
        let is_up = instruction.check_bit(23);
        let has_writeback = instruction.check_bit(21);

        let register_list = instruction.get_bits(0, 15) as u16;
        let register_count = register_list.count_ones();
        let reg_base = instruction.get_bits(16, 19) as usize;

        let mut address = cpu.read_reg(reg_base);
        let final_address;

        if is_up {
            final_address = address.wrapping_add(4 * register_count);
        } else {
            final_address = address.wrapping_sub(4 * register_count);
            if is_preindexed {
                // Pre increment will need to  be one lower
                address = final_address.wrapping_sub(4);
            } else {
                // Post decrement starts one higher than pre decrement, but both end at same address
                address = final_address.wrapping_add(4);
            }
        }

        // ** S bit handling **
        let old_mode = cpu.registers.cpsr.mode();
        let mut swapped_banks = false;

        // This function has the s-bit set, so we need to handle it.
        if register_list.check_bit(15) {
            if is_load {
                // LDM with R15 in transfer list and S bit set (Mode changes)
                cpu.registers.cpsr = cpu.registers.spsr;
            } else {
                // For STM instructions data will be taken from the User bank, so we need to switch to that.
                swapped_banks = cpu.registers.swap_register_banks(old_mode, Mode::User);
            }
        } else {
            swapped_banks = cpu.registers.swap_register_banks(old_mode, Mode::User);
        }

        // Inefficient iteration for now, TODO: Optimise.
        for i in 0..16 {
            if register_list.check_bit(i) {
                if is_preindexed {
                    address = address.wrapping_add(4)
                }

                let reg_dest = i as usize;
                if is_load {
                    let value = bus.read_32(address);
                    cpu.write_reg(reg_dest, value, bus);
                } else {
                    let value = cpu.read_reg(reg_dest);
                    bus.write_32(address, value);
                }

                if !is_preindexed {
                    address = address.wrapping_add(4)
                }
            }
        }

        if swapped_banks {
            // In theory has_writeback should be false, but can't hurt to pre-emptively swap back.
            cpu.registers.swap_register_banks(Mode::User, old_mode);
        }

        if has_writeback {
            cpu.write_reg(reg_base, final_address, bus);
        }
    }
}
