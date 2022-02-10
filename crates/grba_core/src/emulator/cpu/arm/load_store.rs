use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmV4};
use crate::emulator::cpu::common::ShiftType;
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;
use num_traits::FromPrimitive;

impl ArmV4 {
    #[allow(clippy::collapsible_else_if)]
    pub fn single_data_transfer(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        crate::cpu_log!("Executing instruction: Single Data Transfer");
        let (reg_base, reg_dest) = (
            instruction.get_bits(16, 19) as usize,
            instruction.get_bits(12, 15) as usize,
        );
        let is_load = instruction.check_bit(20);
        let is_byte_transfer = instruction.check_bit(22);
        let is_up = instruction.check_bit(23);
        let is_preindexed = instruction.check_bit(24);
        let has_writeback = instruction.check_bit(21);
        let is_immediate = !instruction.check_bit(25);

        let offset = if is_immediate {
            instruction.get_bits(0, 11)
        } else {
            let reg_offset = instruction.get_bits(0, 3) as usize;
            // We are working with a register
            let shift_type = ShiftType::from_u32(instruction.get_bits(5, 6)).unwrap();

            // Immediate Shift
            let shift_amount = instruction.get_bits(7, 11) as u8;
            let (offset, carry) =
                shift_type.perform_shift(cpu.read_reg(reg_offset), shift_amount, cpu.registers.cpsr.carry());

            //TODO: This might cause issues if we have an arithmetic instruction which doesn't set condition codes,
            // as the previous carry value would be overwritten.
            cpu.registers.cpsr.set_carry(carry);
            offset
        };

        let base_address = cpu.read_reg(reg_base);
        let address = if is_preindexed {
            if is_up {
                base_address.wrapping_add(offset)
            } else {
                base_address.wrapping_sub(offset)
            }
        } else {
            base_address
        };

        // Actual Operations:
        if is_load {
            if is_byte_transfer {
                let value = bus.read(address) as u32;
                cpu.write_reg(reg_dest, value, bus);
            } else {
                let value = bus.read_32(address);
                // The byte at the address will always be at bits 0..=7, if unaligned access then the rest will be shifted.
                let final_val = value.rotate_right(8 * (address.get_bits(0, 1)));
                cpu.write_reg(reg_dest, final_val, bus);
            }
        } else {
            //TODO: When R15 is the source register (Rd) of a register store (STR) instruction, the stored
            // value will be address of the instruction plus 12. (Currently it would be +8)
            if is_byte_transfer {
                let data = cpu.read_reg(reg_dest) as u8;
                bus.write(address, data);
            } else {
                let data = cpu.read_reg(reg_dest);
                // TODO: Check if force align is necessary
                bus.write_32(address & 0xFFFF_FFFC, data);
            }
        }

        // TODO: verify if we interpreted the post index correctly.
        // Resolve post indexing and write back
        if !is_preindexed {
            let addr = if is_up { base_address.wrapping_add(offset) } else { base_address.wrapping_sub(offset) };

            cpu.write_reg(reg_base, addr, bus);
        } else if has_writeback {
            cpu.write_reg(reg_base, address, bus);
        }
    }

    pub fn halfword_and_signed_register(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        crate::cpu_log!("Executing instruction: Halfword and Signed Data Transfer Register");
        let is_preindexed = instruction.check_bit(24);
        let is_up = instruction.check_bit(23);
        let has_writeback = instruction.check_bit(21);
        let is_load = instruction.check_bit(20);
        let sh = SwapType::from_u32(instruction.get_bits(5, 6)).unwrap();

        let (reg_base, reg_dest) = (
            instruction.get_bits(16, 19) as usize,
            instruction.get_bits(12, 15) as usize,
        );

        let reg_offset = instruction.get_bits(0, 3) as usize;
        let offset = cpu.read_reg(reg_offset);

        let base_address = cpu.read_reg(reg_base);
        let address = if is_preindexed {
            if is_up {
                base_address.wrapping_add(offset)
            } else {
                base_address.wrapping_sub(offset)
            }
        } else {
            base_address
        };

        Self::halfword_operation(
            cpu,
            instruction,
            bus,
            is_preindexed,
            is_up,
            has_writeback,
            is_load,
            sh,
            reg_base,
            reg_dest,
            offset,
            address,
        )
    }

    pub fn halfword_and_signed_immediate(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        crate::cpu_log!("Executing instruction: Halfword and Signed Data Transfer Immediate");
        let is_preindexed = instruction.check_bit(24);
        let is_up = instruction.check_bit(23);
        let has_writeback = instruction.check_bit(21);
        let is_load = instruction.check_bit(20);
        let sh = SwapType::from_u32(instruction.get_bits(5, 6)).unwrap();

        let (reg_base, reg_dest) = (
            instruction.get_bits(16, 19) as usize,
            instruction.get_bits(12, 15) as usize,
        );
        let base_address = cpu.read_reg(reg_base);

        let offset = (instruction.get_bits(8, 11) >> 4) | instruction.get_bits(0, 3);
        let address = if is_preindexed {
            if is_up {
                base_address.wrapping_add(offset)
            } else {
                base_address.wrapping_sub(offset)
            }
        } else {
            base_address
        };

        Self::halfword_operation(
            cpu,
            instruction,
            bus,
            is_preindexed,
            is_up,
            has_writeback,
            is_load,
            sh,
            reg_base,
            reg_dest,
            offset,
            address,
        )
    }

    #[inline(always)]
    fn halfword_operation(
        cpu: &mut CPU,
        instruction: ArmInstruction,
        bus: &mut Bus,
        is_preindexed: bool,
        is_up: bool,
        has_writeback: bool,
        is_load: bool,
        sh: SwapType,
        reg_base: usize,
        reg_dest: usize,
        offset: u32,
        address: u32,
    ) {
        match sh {
            SwapType::Swp => todo!("Swap instruction in halfword and signed register?: {:#X?}", instruction),
            SwapType::UnsignedU16 => {
                if is_load {
                    let value = bus.read_16(address) as u32;
                    cpu.write_reg(reg_dest, value, bus);
                } else {
                    let value = cpu.read_reg(reg_dest) as u16;
                    bus.write_16(address, value);
                }
            }
            SwapType::Signedi8 => {
                // Load bit *shouldn't* be low, so we'll just ignore it!
                let value = bus.read(address) as i8;
                // Sign extension should take place, but since we're casting from a smaller int to larger int this is done automatically.
                cpu.write_reg(reg_dest, value as i32 as u32, bus);
            }
            SwapType::Signedi16 => {
                // Load bit *shouldn't* be low, so we'll just ignore it!
                let value = bus.read(address) as i16;
                // Sign extension should take place, but since we're casting from a smaller int to larger int this is done automatically.
                cpu.write_reg(reg_dest, value as i32 as u32, bus);
            }
        }

        // Resolve post indexing and write back
        if !is_preindexed {
            let addr = if is_up { address.wrapping_add(offset) } else { address.wrapping_sub(offset) };

            cpu.write_reg(reg_base, addr, bus);
        } else if has_writeback {
            cpu.write_reg(reg_base, address, bus);
        }
    }
}

#[derive(num_derive::FromPrimitive, Debug)]
enum SwapType {
    Swp = 0b00,
    UnsignedU16 = 0b01,
    Signedi8 = 0b10,
    Signedi16 = 0b11,
}
