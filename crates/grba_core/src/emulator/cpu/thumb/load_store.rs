use crate::emulator::bus::Bus;
use crate::emulator::cpu::registers::{LINK_REG, PC_REG, SP_REG};
use crate::emulator::cpu::thumb::{ThumbInstruction, ThumbV4};
use crate::emulator::cpu::CPU;
use crate::utils::{sign_extend32, BitOps};

impl ThumbV4 {
    pub fn pc_relative_load(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let r_d = instruction.get_bits(8, 10) as usize;

        let imm_value = instruction.get_bits(0, 7) << 2;
        // PC value must always be word aligned for this addition
        let pc_value = cpu.registers.pc() & 0xFFFFFFFC;

        let address = pc_value.wrapping_add(imm_value as u32);
        // Read the value at the specified address and write it to r_d
        cpu.write_reg(r_d, bus.read_32(address, cpu), bus);
    }

    pub fn load_store_with_reg_offset(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let is_load = instruction.check_bit(11);
        let is_byte_transfer = instruction.check_bit(10);

        let r_offset = instruction.get_bits(6, 8) as usize;
        let r_base = instruction.get_bits(3, 5) as usize;
        let r_d = instruction.get_bits(0, 2) as usize;

        let target_addr = cpu.read_reg(r_base).wrapping_add(cpu.read_reg(r_offset));

        Self::load_or_store_value(cpu, bus, is_load, is_byte_transfer, r_d, target_addr)
    }

    pub fn load_store_with_immediate_offset(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let is_byte_transfer = instruction.check_bit(12);
        let is_load = instruction.check_bit(11);

        // If it is a word transfer then the offset is left shifted by 2
        let offset = (instruction.get_bits(6, 10) as u32) << ((!is_byte_transfer as u32) * 2);
        let r_base = instruction.get_bits(3, 5) as usize;
        let r_d = instruction.get_bits(0, 2) as usize;

        let target_addr = cpu.read_reg(r_base).wrapping_add(offset);

        Self::load_or_store_value(cpu, bus, is_load, is_byte_transfer, r_d, target_addr)
    }

    pub fn load_store_sign_extended_byte_halfword(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let h_flag = instruction.check_bit(11);
        let is_sign_extended = instruction.check_bit(10);

        let r_offset = instruction.get_bits(6, 8) as usize;
        let r_base = instruction.get_bits(3, 5) as usize;
        let r_d = instruction.get_bits(0, 2) as usize;

        let target_addr = cpu.read_reg(r_base).wrapping_add(cpu.read_reg(r_offset));

        if is_sign_extended {
            if h_flag {
                cpu.write_reg(r_d, sign_extend32(bus.read_16(target_addr, cpu) as u32, 8) as u32, bus);
            } else {
                cpu.write_reg(r_d, sign_extend32(bus.read(target_addr, cpu) as u32, 8) as u32, bus);
            }
        } else {
            if h_flag {
                cpu.write_reg(r_d, bus.read_16(target_addr, cpu) as u32, bus);
            } else {
                bus.write_16(target_addr, cpu.read_reg(r_d) as u16);
            }
        }
    }

    pub fn load_store_halfword(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let is_load = instruction.check_bit(11);

        let offset = (instruction.get_bits(6, 10) as u32) << 1;
        let r_base = instruction.get_bits(3, 5) as usize;
        let r_d = instruction.get_bits(0, 2) as usize;

        let target_addr = cpu.read_reg(r_base).wrapping_add(offset);

        if is_load {
            cpu.write_reg(r_d, bus.read_16(target_addr, cpu) as u32, bus);
        } else {
            bus.write_16(target_addr, cpu.read_reg(r_d) as u16);
        }
    }

    pub fn sp_relative_load_store(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let is_load = instruction.check_bit(11);

        let r_d = instruction.get_bits(8, 10) as usize;
        let offset = (instruction.get_bits(0, 7) as u32) << 2;

        let target_addr = cpu.read_reg(SP_REG).wrapping_add(offset);

        if is_load {
            cpu.write_reg(r_d, bus.read_32(target_addr, cpu), bus);
        } else {
            bus.write_32(target_addr, cpu.read_reg(r_d));
        }
    }

    pub fn load_address(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        let source_is_sp = instruction.check_bit(11);

        let r_d = instruction.get_bits(8, 10) as usize;
        let constant = (instruction.get_bits(0, 7) as u32) << 2;

        let load_value = if source_is_sp { cpu.read_reg(SP_REG) } else { cpu.read_reg(PC_REG) & 0xFFFF_FFFC };

        let final_value = load_value.wrapping_add(constant);

        cpu.write_reg(r_d, final_value, bus);
    }

    pub fn add_offset_to_stack_pointer(cpu: &mut CPU, instruction: ThumbInstruction, bus: &mut Bus) {
        //TODO: We should be able to do this by just casting the offset range (0, 7) to (as i8 as u32)
        let offset_is_negative = instruction.check_bit(7);
        let offset = (instruction.get_bits(0, 6) as u32) << 2;

        let new_sp = if offset_is_negative {
            cpu.read_reg(SP_REG).wrapping_sub(offset)
        } else {
            cpu.read_reg(SP_REG).wrapping_add(offset)
        };

        cpu.write_reg(SP_REG, new_sp, bus);
    }

    #[inline(always)]
    fn load_or_store_value(
        cpu: &mut CPU,
        bus: &mut Bus,
        is_load: bool,
        is_byte_transfer: bool,
        r_d: usize,
        target_addr: u32,
    ) {
        if is_load {
            if is_byte_transfer {
                cpu.write_reg(r_d, bus.read(target_addr, cpu) as u32, bus);
            } else {
                cpu.write_reg(r_d, bus.read_32(target_addr, cpu), bus);
            }
        } else {
            if is_byte_transfer {
                bus.write(target_addr, cpu.read_reg(r_d) as u8);
            } else {
                bus.write_32(target_addr, cpu.read_reg(r_d));
            }
        }
    }
}
