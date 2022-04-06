use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmV4};
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;

impl ArmV4 {
    pub fn single_data_swap(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        let is_byte_read = instruction.check_bit(22);
        let (reg_base, reg_src, reg_dst) = (
            instruction.get_bits(16, 19) as usize,
            instruction.get_bits(0, 3) as usize,
            instruction.get_bits(12, 15) as usize,
        );

        let source_content = cpu.read_reg(reg_src);
        let base_address = cpu.read_reg(reg_base);

        if is_byte_read {
            let current_mem = bus.read(base_address, cpu);

            bus.write(base_address, source_content as u8);
            cpu.write_reg(reg_dst, current_mem as u32, bus);
        } else {
            let rotate_amount = base_address.get_bits(0, 1);
            let base_address = base_address;
            let current_mem = bus.read_32(base_address, cpu).rotate_right(8 * rotate_amount);

            bus.write_32(base_address, source_content);
            cpu.write_reg(reg_dst, current_mem, bus);
        }
    }
}
