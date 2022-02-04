use crate::bus::Bus;
use crate::cpu::arm::{ArmInstruction, ArmV4T};
use crate::cpu::CPU;
use crate::utils::{check_bit, get_bits};

impl ArmV4T {
    pub fn single_data_swap(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        let is_byte_read = check_bit(instruction, 22);
        let (reg_base, reg_src, reg_dst) = (
            get_bits(instruction, 16, 19) as usize,
            get_bits(instruction, 0, 3) as usize,
            get_bits(instruction, 12, 15) as usize,
        );

        let source_content = cpu.read_reg(reg_src);
        let base_address = cpu.read_reg(reg_base);

        if is_byte_read {
            let current_mem = bus.read(base_address);

            bus.write(base_address, source_content as u8);
            cpu.write_reg(reg_dst, current_mem as u32);
        } else {
            let current_mem = bus.read_32(base_address);

            bus.write_32(base_address, source_content);
            cpu.write_reg(reg_dst, current_mem);
        }
    }
}
