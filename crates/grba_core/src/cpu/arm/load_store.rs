use crate::bus::Bus;
use crate::cpu::arm::{ArmInstruction, ArmV4T, ShiftType};
use crate::cpu::CPU;
use crate::utils::BitOps;
use num_traits::FromPrimitive;

impl ArmV4T {
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
}
