use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmV4};
use crate::emulator::cpu::common::ShiftType;
use crate::emulator::cpu::registers::PC_REG;
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;
use num_traits::FromPrimitive;

impl ArmV4 {
    #[allow(clippy::collapsible_else_if)]
    pub fn single_data_transfer(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
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
            // No flags are set by the LDR/STR instructions.
            let (offset, _) =
                shift_type.perform_shift(cpu.read_reg(reg_offset), shift_amount, cpu.registers.cpsr.carry());

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
                let value = bus.read(address & 0xFFFF_FFFC, cpu) as u32;
                cpu.write_reg(reg_dest, value, bus);
            } else {
                let value = bus.read_32(address & 0xFFFF_FFFC, cpu);
                // The byte at the address will always be at bits 0..=7, if unaligned access then the rest will be shifted.
                let final_val = value.rotate_right(8 * (address.get_bits(0, 1)));
                cpu.write_reg(reg_dest, final_val, bus);
            }
        } else {
            // For store instructions, when R15 is specified in r_d it should be 3 words ahead of the current instruction.
            // Usually it's +2, thus we need to temporarily add 4 to the address
            cpu.registers.general_purpose[PC_REG] += 4;

            if is_byte_transfer {
                let data = cpu.read_reg(reg_dest) as u8;
                bus.write(address, data);
            } else {
                let data = cpu.read_reg(reg_dest);
                // Force align the address
                bus.write_32(address & 0xFFFF_FFFC, data);
            }

            cpu.registers.general_purpose[PC_REG] -= 4;
        }

        // No writeback occurs if the base and destination register are the same AND it's a load instruction.
        if (is_load && reg_base != reg_dest) || !is_load {
            // Resolve post indexing and write back
            if !is_preindexed {
                let addr = if is_up { base_address.wrapping_add(offset) } else { base_address.wrapping_sub(offset) };

                cpu.write_reg(reg_base, addr, bus);
            } else if has_writeback {
                cpu.write_reg(reg_base, address, bus);
            }
        }
    }

    pub fn halfword_and_signed_register(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
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
                    let value = bus.read_16(address & 0xFFFF_FFFE, cpu) as u32;
                    // For ARMv4 we have to force align and rotate the read value on unaligned reads, only force align for ARMv5+
                    let final_val = value.rotate_right(8 * (address.check_bit(0) as u32));
                    cpu.write_reg(reg_dest, final_val, bus);
                } else {
                    let value = cpu.read_reg(reg_dest) as u16;
                    bus.write_16(address & 0xFFFF_FFFE, value);
                }
            }
            SwapType::Signedi8 => {
                // Load bit *shouldn't* be low, so we'll just ignore it!
                // Sign extension should take place, but since we're casting from a smaller int to larger int this is done automatically.
                let value = bus.read(address, cpu) as i8 as u32;

                cpu.write_reg(reg_dest, value, bus);
            }
            // Special case, if we're unaligned then only the odd byte is read, and then sign-extended
            SwapType::Signedi16 if address.check_bit(0) => {
                // Load bit *shouldn't* be low, so we'll just ignore it!
                // Sign extension should take place, but since we're casting from a smaller int to larger int this is done automatically.
                let value = bus.read(address, cpu) as i8 as u32;

                cpu.write_reg(reg_dest, value as u32, bus);
            }
            SwapType::Signedi16 => {
                // Load bit *shouldn't* be low, so we'll just ignore it!
                // Sign extension should take place, but since we're casting from a smaller int to larger int this is done automatically.
                let value = bus.read_16(address & 0xFFFF_FFFE, cpu) as i16 as u32;

                cpu.write_reg(reg_dest, value, bus);
            }
        }

        if (is_load && reg_base != reg_dest) || !is_load {
            // Resolve post indexing and write back
            if !is_preindexed {
                let addr = if is_up { address.wrapping_add(offset) } else { address.wrapping_sub(offset) };

                cpu.write_reg(reg_base, addr, bus);
            } else if has_writeback {
                cpu.write_reg(reg_base, address, bus);
            }
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
