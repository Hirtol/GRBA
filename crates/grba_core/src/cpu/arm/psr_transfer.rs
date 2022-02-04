use crate::bus::Bus;
use crate::cpu::arm::ArmInstruction;
use crate::cpu::CPU;
use crate::utils::{check_bit, get_bits};

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

impl CPU {
    /// Transfer PSR contents to a register
    pub fn mrs_trans_psr_reg(&mut self, instruction: ArmInstruction, _bus: &mut Bus) {
        let r_d = get_bits(instruction, 12, 15) as usize;
        let source_psr: Psr = check_bit(instruction, 22).into();

        let contents = match source_psr {
            Psr::Cpsr => self.registers.cpsr,
            Psr::Spsr => self.registers.spsr,
        };

        self.write_reg(r_d, contents.as_raw());
    }

    /// Transfer a register contents to PSR.
    ///
    /// Done as a separate match as the requisite bit is not part of our LUT index.
    pub fn msr_match(&mut self, instruction: ArmInstruction, bus: &mut Bus) {
        if check_bit(instruction, 16) {
            self.msr_trans_reg_psr(instruction, bus);
        } else {
            self.msr_trans_reg_imm_psr_flag(instruction, bus);
        }
    }

    /// Transfer register contents to PSR.
    ///
    /// Should not be called in User mode
    pub fn msr_trans_reg_psr(&mut self, instruction: ArmInstruction, _bus: &mut Bus) {
        let r_m = get_bits(instruction, 0, 3) as usize;
        let dest_psr: Psr = check_bit(instruction, 22).into();
        let value = self.read_reg(r_m);

        match dest_psr {
            Psr::Cpsr => self.registers.cpsr = value.into(),
            Psr::Spsr => self.registers.spsr = value.into(),
        }
    }

    /// Transfer register contents or immediate value to PSR flag bits only
    pub fn msr_trans_reg_imm_psr_flag(&mut self, instruction: ArmInstruction, _bus: &mut Bus) {
        let dest_psr: Psr = check_bit(instruction, 22).into();
        let immediate = check_bit(instruction, 25);

        let update_value = if immediate {
            // Shift amount is 0 extended to 32 bits, then rotated right by `rotate amount * 2`
            let rotate = get_bits(instruction, 8, 11) * 2;
            let imm = get_bits(instruction, 0, 7) as u32;
            imm.rotate_right(rotate)
        } else {
            let r_m = get_bits(instruction, 0, 3) as usize;
            self.read_reg(r_m)
        };

        match dest_psr {
            Psr::Cpsr => self.registers.cpsr.update_control_flags(update_value),
            Psr::Spsr => self.registers.spsr.update_control_flags(update_value),
        }
    }
}
