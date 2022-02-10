//! Contains values common for the ARM and THUMB instruction sets.

use crate::emulator::cpu::CPU;
use crate::utils::BitOps;

#[derive(Debug, Eq, PartialEq, Copy, Clone, num_derive::FromPrimitive)]
pub enum ShiftType {
    LogicalLeft = 0b00,
    LogicalRight = 0b01,
    ArithRight = 0b10,
    RotateRight = 0b11,
}

impl ShiftType {
    /// Performs the specified shift operation on the given value.
    /// Will return the shifted value, as well as the carry flag.
    #[inline]
    pub fn perform_shift(self, value: u32, shift_amount: u8, current_carry: bool) -> (u32, bool) {
        match self {
            ShiftType::LogicalLeft => {
                let carry = if shift_amount == 0 { current_carry } else { value.check_bit(32 - shift_amount) };

                let shifted = value << shift_amount;
                // Least significant bit that is shifted out goes to the carry flag
                (shifted, carry)
            }
            ShiftType::LogicalRight => {
                // ARM thought it'd be fun to allow 32-bit shifts to the right with different carry behaviour... yay
                if shift_amount < 32 {
                    let carry = value.check_bit(shift_amount.saturating_sub(1));
                    let shifted = value >> shift_amount;
                    (shifted, carry)
                } else {
                    let carry = value.check_bit(31);
                    (0, carry)
                }
            }
            ShiftType::ArithRight => {
                if shift_amount < 32 {
                    let carry = value.check_bit(shift_amount.saturating_sub(1));
                    // We cast to an i32 to get an arithmetic shift, then cast back.
                    let shifted = ((value as i32) >> shift_amount) as u32;

                    (shifted, carry)
                } else {
                    let carry = value.check_bit(31);
                    // Since we're doing signed extension we either return nothing at all or all ones.
                    let shifted = if carry { u32::MAX } else { 0 };
                    (shifted, carry)
                }
            }
            ShiftType::RotateRight => {
                if shift_amount == 0 {
                    let carry_flag = (current_carry as u32) << 31;
                    // Carry flag is appended and everything is shifted by one position
                    (carry_flag | (value >> 1), value.check_bit(0))
                } else {
                    let carry = value.check_bit(shift_amount.saturating_sub(1));
                    let shifted = value.rotate_right(shift_amount as u32);
                    (shifted, carry)
                }
            }
        }
    }
}

impl CPU {
    #[inline(always)]
    pub(crate) fn set_logical_flags(&mut self, value: u32, carry: bool) {
        self.registers.cpsr.set_zero(value == 0);
        self.registers.cpsr.set_carry(carry);
        self.registers.cpsr.set_sign(value.check_bit(31));
    }

    #[inline(always)]
    pub(crate) fn set_arithmetic_flags(&mut self, value: u32, carry: bool, overflow: bool) {
        self.set_logical_flags(value, carry);
        self.registers.cpsr.set_overflow(overflow);
    }
}
