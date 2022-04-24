use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmV4};
use crate::emulator::cpu::registers::{Mode, PC_REG};
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;

impl ArmV4 {
    #[grba_lut_generate::create_lut(u32, HAS_WRITEBACK = 5, PSR_OR_USER = 6, IS_UP = 7, PRE_INDEXED = 8)]
    pub fn block_data_transfer_store<
        const HAS_WRITEBACK: bool,
        const PSR_OR_USER: bool,
        const IS_UP: bool,
        const PRE_INDEXED: bool,
    >(
        cpu: &mut CPU,
        instruction: ArmInstruction,
        bus: &mut Bus,
    ) {
        // For the duration of this instruction PC will be 12 ahead instead of just 8.
        cpu.registers.general_purpose[PC_REG] += 4;
        Self::block_data_transfer::<PSR_OR_USER, false, PRE_INDEXED, IS_UP, HAS_WRITEBACK>(cpu, instruction, bus);
        cpu.registers.general_purpose[PC_REG] -= 4;
    }

    // Normal bit locations:
    // * HAS_WRITEBACK: 21
    // * PSR_OR_USER: 22
    // * IS_UP: 23
    // * PRE_INDEXED: 24
    #[grba_lut_generate::create_lut(u32, HAS_WRITEBACK = 5, PSR_OR_USER = 6, IS_UP = 7, PRE_INDEXED = 8)]
    pub fn block_data_transfer_load<
        const HAS_WRITEBACK: bool,
        const PSR_OR_USER: bool,
        const IS_UP: bool,
        const PRE_INDEXED: bool,
    >(
        cpu: &mut CPU,
        instruction: ArmInstruction,
        bus: &mut Bus,
    ) {
        Self::block_data_transfer::<PSR_OR_USER, true, PRE_INDEXED, IS_UP, HAS_WRITEBACK>(cpu, instruction, bus);
    }

    #[inline(always)]
    fn block_data_transfer<
        const PSR_OR_USER: bool,
        const IS_LOAD: bool,
        const PRE_INDEXED: bool,
        const IS_UP: bool,
        const HAS_WRITEBACK: bool,
    >(
        cpu: &mut CPU,
        instruction: ArmInstruction,
        bus: &mut Bus,
    ) {
        let register_list = instruction.get_bits(0, 15) as u16;

        if PSR_OR_USER {
            Self::block_data_transfer_with_s_bit(
                cpu,
                instruction,
                bus,
                register_list,
                IS_LOAD,
                PRE_INDEXED,
                IS_UP,
                HAS_WRITEBACK,
            );
        } else {
            Self::block_data_transfer_without_s_bit(
                cpu,
                instruction,
                bus,
                register_list,
                IS_LOAD,
                PRE_INDEXED,
                IS_UP,
                HAS_WRITEBACK,
            );
        }
    }

    #[inline(always)]
    fn block_data_transfer_without_s_bit(
        cpu: &mut CPU,
        instruction: ArmInstruction,
        bus: &mut Bus,
        mut register_list: u16,
        is_load: bool,
        is_preindexed: bool,
        is_up: bool,
        mut has_writeback: bool,
    ) {
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
        Self::iterate_registers(cpu, bus, is_load, register_list, address);

        if has_writeback {
            cpu.write_reg(reg_base, writeback_address, bus);
        }
    }

    #[inline(always)]
    fn block_data_transfer_with_s_bit(
        cpu: &mut CPU,
        instruction: ArmInstruction,
        bus: &mut Bus,
        mut register_list: u16,
        is_load: bool,
        is_preindexed: bool,
        is_up: bool,
        mut has_writeback: bool,
    ) {
        let register_count = register_list.count_ones();
        let reg_base = instruction.get_bits(16, 19) as usize;

        let (writeback_address, address) =
            Self::calculate_addresses(is_preindexed, is_up, register_count, cpu.read_reg(reg_base));

        // Handle edge case of empty register list. (Note, pretty sure no real game uses this edge case, can probably remove)
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
            cpu.registers.write_cpsr(cpu.registers.spsr, bus);
        } else {
            // For STM instructions data will be taken from the User bank, so we need to switch to that.
            swapped_banks = cpu.registers.swap_register_banks(old_mode, Mode::User, false);
        }

        // Handle all registers
        Self::iterate_registers(cpu, bus, is_load, register_list, address);

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
    const fn calculate_addresses(
        is_preindexed: bool,
        is_up: bool,
        register_count: u32,
        start_address: u32,
    ) -> (u32, u32) {
        if is_up {
            if register_count != 0 {
                let final_address = start_address.wrapping_add(4 * register_count);
                let start_address = if is_preindexed { start_address + 4 } else { start_address };

                (final_address, start_address)
            } else {
                // Handle edge case where register list is empty. If we're pre-indexed we do a branchless initial add.
                (start_address + 0x40, start_address + (4 * is_preindexed as u32))
            }
        } else {
            let final_address = if register_count != 0 {
                start_address.wrapping_sub(4 * register_count)
            } else {
                // Handle edge case where register list is empty (Note: Probably not worth keeping for future optimisation)
                start_address - 0x40
            };

            let start_address = if is_preindexed {
                // Pre decrement will start at the final address
                final_address
            } else {
                // Post decrement starts one higher than pre decrement, but both end at same address
                final_address.wrapping_add(4)
            };

            (final_address, start_address)
        }
    }

    #[inline(always)]
    fn iterate_registers(cpu: &mut CPU, bus: &mut Bus, is_load: bool, register_list: u16, mut address: u32) {
        if is_load {
            for i in 0..16 {
                if register_list.check_bit(i) {
                    let value = bus.read_32(address, cpu);
                    cpu.write_reg(i as usize, value, bus);

                    address = address.wrapping_add(4)
                }
            }
        } else {
            for i in 0..16 {
                if register_list.check_bit(i) {
                    let value = cpu.read_reg(i as usize);
                    bus.write_32(address, value);

                    address = address.wrapping_add(4)
                }
            }
        }
    }
}
