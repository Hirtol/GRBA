use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmV4};
use crate::emulator::cpu::registers::PSR;
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;

enum Psr {
    Cpsr,
    Spsr,
}

impl ArmV4 {
    /// Transfer PSR contents to a register
    pub fn mrs_trans_psr_reg(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        let r_d = instruction.get_bits(12, 15) as usize;
        let source_psr: Psr = instruction.check_bit(22).into();

        let contents = match source_psr {
            Psr::Cpsr => cpu.registers.cpsr,
            Psr::Spsr => cpu.registers.spsr,
        };

        cpu.write_reg(r_d, contents.as_raw(), bus);
    }

    /// Transfer register contents to PSR.
    ///
    /// Should not be called in User mode
    pub fn msr_immediate(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        // Shift amount is 0 extended to 32 bits, then rotated right by `rotate amount * 2`
        let rotate = instruction.get_bits(8, 11) * 2;
        let imm = instruction.get_bits(0, 7) as u32;
        let new_value = imm.rotate_right(rotate);

        Self::msr_common(cpu, bus, instruction, new_value);
    }

    /// Transfer register contents to PSR.
    ///
    /// Should not be called in User mode
    pub fn msr_register(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        let r_m = instruction.get_bits(0, 3) as usize;
        let new_value = cpu.read_reg(r_m);

        Self::msr_common(cpu, bus, instruction, new_value);
    }

    fn msr_common(cpu: &mut CPU, bus: &mut Bus, instruction: ArmInstruction, new_value: u32) {
        let dest_psr: Psr = instruction.check_bit(22).into();
        let field_mask = instruction.get_bits(16, 19);

        let mut cur_psr_value = match dest_psr {
            Psr::Cpsr => cpu.registers.cpsr.as_raw(),
            Psr::Spsr => cpu.registers.spsr.as_raw(),
        };

        // Control bits
        if field_mask.check_bit(0) {
            cur_psr_value = (cur_psr_value & 0xFFFFFF00) | (new_value & 0xFF);
        }
        // Extension bits
        if field_mask.check_bit(1) {
            cur_psr_value = (cur_psr_value & 0xFFFF00FF) | (new_value & 0xFF00);
        }
        // Status bits
        if field_mask.check_bit(2) {
            cur_psr_value = (cur_psr_value & 0xFF00FFFF) | (new_value & 0xFF0000);
        }
        // Flag bits
        if field_mask.check_bit(3) {
            cur_psr_value = (cur_psr_value & 0x00FFFFFF) | (new_value & 0xFF000000);
        }

        match dest_psr {
            Psr::Cpsr => cpu.registers.write_cpsr(PSR::from_raw(cur_psr_value), bus),
            Psr::Spsr => cpu.registers.spsr = PSR::from_raw(cur_psr_value),
        };
    }
}

impl From<bool> for Psr {
    fn from(val: bool) -> Self {
        if val {
            Psr::Spsr
        } else {
            Psr::Cpsr
        }
    }
}
