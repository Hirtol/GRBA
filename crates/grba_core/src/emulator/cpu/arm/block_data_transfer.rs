use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmV4};
use crate::emulator::cpu::registers::{Mode, PC_REG};
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;

impl ArmV4 {
    pub fn block_data_transfer_store(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        // For the duration of this instruction PC will be 12 ahead instead of just 8.
        cpu.registers.general_purpose[PC_REG] += 4;
        Self::block_data_transfer(cpu, instruction, bus, false);
        cpu.registers.general_purpose[PC_REG] -= 4;
    }

    pub fn block_data_transfer_load(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        Self::block_data_transfer(cpu, instruction, bus, true);
    }

    #[inline(always)]
    fn block_data_transfer(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus, is_load: bool) {
        let psr_or_user = instruction.check_bit(22);
        let register_list = instruction.get_bits(0, 15) as u16;

        if psr_or_user {
            Self::block_data_transfer_with_s_bit(cpu, instruction, bus, is_load, register_list);
        } else {
            Self::block_data_transfer_without_s_bit(cpu, instruction, bus, is_load, register_list);
        }
    }

    #[inline]
    fn block_data_transfer_without_s_bit(
        cpu: &mut CPU,
        instruction: ArmInstruction,
        bus: &mut Bus,
        is_load: bool,
        mut register_list: u16,
    ) {
        let is_preindexed = instruction.check_bit(24);
        let is_up = instruction.check_bit(23);
        let mut has_writeback = instruction.check_bit(21);

        let register_count = register_list.count_ones();
        let reg_base = instruction.get_bits(16, 19) as usize;

        // Handle edge case of empty register list.
        // Empty register list is interpreted as PC_REG being transferred.
        if register_list == 0 {
            register_list = 1 << PC_REG;
        }

        let (writeback_address, address) =
            Self::calculate_addresses(is_preindexed, is_up, register_count, cpu.read_reg(reg_base));

        // Edge cases for where the base register is included in the store/load register list
        if register_list.check_bit(reg_base as u8) {
            if is_load {
                // In ARMv4 writeback gets disabled, note that this is different from ARMv5
                has_writeback = false;
            } else {
                // In ARMv4 we store the old base if the register is *first* in the list, otherwise store new base
                // In ARMv5 we always store the new base
                let registers = register_list << (15 - reg_base);

                // Is not first in the list
                if registers.count_ones() != 1 {
                    cpu.write_reg(reg_base, writeback_address, bus);
                }
            }
        }

        // Handle all registers
        Self::iterate_registers(cpu, bus, is_load, register_list, is_preindexed, address);

        if has_writeback {
            cpu.write_reg(reg_base, writeback_address, bus);
        }
    }

    #[inline]
    fn block_data_transfer_with_s_bit(
        cpu: &mut CPU,
        instruction: ArmInstruction,
        bus: &mut Bus,
        is_load: bool,
        mut register_list: u16,
    ) {
        let is_preindexed = instruction.check_bit(24);
        let is_up = instruction.check_bit(23);
        let mut has_writeback = instruction.check_bit(21);

        let register_count = register_list.count_ones();
        let reg_base = instruction.get_bits(16, 19) as usize;

        let (writeback_address, address) =
            Self::calculate_addresses(is_preindexed, is_up, register_count, cpu.read_reg(reg_base));

        // Handle edge case of empty register list.
        // Empty register list is interpreted as PC_REG being transferred.
        if register_list == 0 {
            register_list = 1 << PC_REG;
        }

        // Edge cases for where the base register is included in the store/load register list
        if register_list.check_bit(reg_base as u8) {
            if is_load {
                // In ARMv4 writeback gets disabled, note that this is different from ARMv5
                has_writeback = false;
            } else {
                // In ARMv4 we store the old base if the register is *first* in the list, otherwise store new base
                // In ARMv5 we always store the new base
                let registers = register_list << (15 - reg_base);
                // Is not first in the list
                if registers.count_ones() != 1 {
                    cpu.write_reg(reg_base, writeback_address, bus);
                }
            }
        }

        // ** S bit handling **
        let old_mode = cpu.registers.cpsr.mode();
        let mut swapped_banks = false;

        // This function has the s-bit set, so we need to handle it.
        if register_list.check_bit(15) && is_load {
            // LDM with R15 in transfer list and S bit set (Mode changes)
            cpu.registers.write_cpsr(cpu.registers.spsr);
        } else {
            // For STM instructions data will be taken from the User bank, so we need to switch to that.
            swapped_banks = cpu.registers.swap_register_banks(old_mode, Mode::User, false);
        }

        // Handle all registers
        Self::iterate_registers(cpu, bus, is_load, register_list, is_preindexed, address);

        if swapped_banks {
            // In theory has_writeback should be false, but can't hurt to pre-emptively swap back.
            cpu.registers.swap_register_banks(Mode::User, old_mode, false);
        }

        if has_writeback {
            cpu.write_reg(reg_base, writeback_address, bus);
        }
    }

    /// Calculate the `(writeback_address, start_address)` values for a block data transfer.
    #[inline(always)]
    fn calculate_addresses(is_preindexed: bool, is_up: bool, register_count: u32, start_address: u32) -> (u32, u32) {
        if is_up {
            if register_count != 0 {
                (start_address.wrapping_add(4 * register_count), start_address)
            } else {
                // Handle edge case where register list is empty (Note: Probably not worth keeping for future optimisation)
                (start_address + 0x40, start_address)
            }
        } else {
            let final_address = if register_count != 0 {
                start_address.wrapping_sub(4 * register_count)
            } else {
                // Handle edge case where register list is empty (Note: Probably not worth keeping for future optimisation)
                start_address - 0x40
            };

            let start_address = if is_preindexed {
                // Pre increment will need to  be one lower
                final_address.wrapping_sub(4)
            } else {
                // Post decrement starts one higher than pre decrement, but both end at same address
                final_address.wrapping_add(4)
            };

            (final_address, start_address)
        }
    }

    #[inline(always)]
    fn iterate_registers(
        cpu: &mut CPU,
        bus: &mut Bus,
        is_load: bool,
        register_list: u16,
        is_preindexed: bool,
        mut address: u32,
    ) {
        // Inefficient iteration for now, TODO: Optimise.
        for i in 0..16 {
            if register_list.check_bit(i) {
                if is_preindexed {
                    address = address.wrapping_add(4)
                }

                let reg_dest = i as usize;
                if is_load {
                    let value = bus.read_32(address, cpu);
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
    }
}
