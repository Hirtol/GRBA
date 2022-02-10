use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmV4T};
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;

impl ArmV4T {
    pub fn single_data_swap(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        crate::cpu_log!("Executing instruction: Single Data Swap");
        let is_byte_read = instruction.check_bit(22);
        let (reg_base, reg_src, reg_dst) = (
            instruction.get_bits(16, 19) as usize,
            instruction.get_bits(0, 3) as usize,
            instruction.get_bits(12, 15) as usize,
        );

        let source_content = cpu.read_reg(reg_src);
        let base_address = cpu.read_reg(reg_base);

        if is_byte_read {
            let current_mem = bus.read(base_address);

            bus.write(base_address, source_content as u8);
            cpu.write_reg(reg_dst, current_mem as u32, bus);
        } else {
            let current_mem = bus.read_32(base_address);

            bus.write_32(base_address, source_content);
            cpu.write_reg(reg_dst, current_mem, bus);
        }
    }
}
