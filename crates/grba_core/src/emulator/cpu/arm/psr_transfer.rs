use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmV4T};
use crate::emulator::cpu::CPU;
use crate::utils::BitOps;

enum Psr {
    Cpsr,
    Spsr,
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

impl ArmV4T {
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

    /// Transfer a register contents to PSR.
    ///
    /// Done as a separate match as the requisite bit is not part of our LUT index.
    pub fn msr_match(cpu: &mut CPU, instruction: ArmInstruction, bus: &mut Bus) {
        if instruction.check_bit(16) {
            ArmV4T::msr_trans_reg_psr(cpu, instruction, bus);
        } else {
            ArmV4T::msr_trans_reg_imm_psr_flag(cpu, instruction, bus);
        }
    }

    /// Transfer register contents to PSR.
    ///
    /// Should not be called in User mode
    pub fn msr_trans_reg_psr(cpu: &mut CPU, instruction: ArmInstruction, _bus: &mut Bus) {
        let r_m = instruction.get_bits(0, 3) as usize;
        let dest_psr: Psr = instruction.check_bit(22).into();
        let value = cpu.read_reg(r_m);

        match dest_psr {
            Psr::Cpsr => cpu.registers.cpsr = value.into(),
            Psr::Spsr => cpu.registers.spsr = value.into(),
        }
    }

    /// Transfer register contents or immediate value to PSR flag bits only
    pub fn msr_trans_reg_imm_psr_flag(cpu: &mut CPU, instruction: ArmInstruction, _bus: &mut Bus) {
        let dest_psr: Psr = instruction.check_bit(22).into();
        let immediate = instruction.check_bit(25);

        let update_value = if immediate {
            // Shift amount is 0 extended to 32 bits, then rotated right by `rotate amount * 2`
            let rotate = instruction.get_bits(8, 11) * 2;
            let imm = instruction.get_bits(0, 7) as u32;
            imm.rotate_right(rotate)
        } else {
            let r_m = instruction.get_bits(0, 3) as usize;
            cpu.read_reg(r_m)
        };

        match dest_psr {
            Psr::Cpsr => cpu.registers.cpsr.update_control_flags(update_value),
            Psr::Spsr => cpu.registers.spsr.update_control_flags(update_value),
        }
    }
}
