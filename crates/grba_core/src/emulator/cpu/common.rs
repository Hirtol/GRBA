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
    pub(crate) fn set_zero_and_sign(&mut self, value: u32) {
        self.registers.cpsr.set_zero(value == 0);
        self.registers.cpsr.set_sign(value.check_bit(31));
    }

    #[inline(always)]
    pub(crate) fn set_logical_flags(&mut self, value: u32, carry: bool) {
        self.set_zero_and_sign(value);
        self.registers.cpsr.set_carry(carry);
    }

    #[inline(always)]
    pub(crate) fn set_arithmetic_flags(&mut self, value: u32, carry: bool, overflow: bool) {
        self.set_logical_flags(value, carry);
        self.registers.cpsr.set_overflow(overflow);
    }
}

pub mod common_behaviour {
    use crate::emulator::bus::Bus;
    use crate::emulator::cpu::registers::{State, PC_REG};
    use crate::emulator::cpu::CPU;
    use crate::utils::{has_sign_overflowed, BitOps};
    use num_traits::FromPrimitive;

    /// Defines the `add` instruction behaviour for both the ARM and THUMB modes.
    ///
    /// The `write_flags` parameter is used to determine whether the flags should be written, will be an arithmetic write.
    #[inline]
    pub fn add(cpu: &mut CPU, op1: u32, op2: u32, write_flags: bool) -> u32 {
        let (result, carry) = op1.overflowing_add(op2);

        if write_flags {
            cpu.set_arithmetic_flags(result, carry, has_sign_overflowed(op1, op2, result));
        }

        result
    }

    /// Defines the `sub` instruction behaviour for both the ARM and THUMB modes.
    ///
    /// The `write_flags` parameter is used to determine whether the flags should be written to, it will be an arithmetic write
    #[inline]
    pub fn sub(cpu: &mut CPU, op1: u32, op2: u32, write_flags: bool) -> u32 {
        let (result, carry) = op1.overflowing_sub(op2);

        if write_flags {
            cpu.set_arithmetic_flags(result, carry, has_sign_overflowed(op1, op2, result));
        }

        result
    }

    #[inline]
    pub fn adc(cpu: &mut CPU, op1: u32, op2: u32, write_flags: bool) -> u32 {
        // We don't use overflowing_add as we need to do a second add immediately, cheaper to check the bit after.
        let full_result = op1 as u64 + op2 as u64 + cpu.registers.cpsr.carry() as u64;
        let result = full_result as u32;

        if write_flags {
            cpu.set_arithmetic_flags(result, full_result.check_bit(32), has_sign_overflowed(op1, op2, result));
        }

        result
    }

    #[inline]
    pub fn sbc(cpu: &mut CPU, op1: u32, op2: u32, write_flags: bool) -> u32 {
        let to_subtract = op2 as u64 - (!cpu.registers.cpsr.carry()) as u64;
        let (full_result, carry) = (op1 as u64).overflowing_sub(to_subtract);
        let result = full_result as u32;

        if write_flags {
            cpu.set_arithmetic_flags(result, carry, has_sign_overflowed(op1, op2, result));
        }

        result
    }

    /// Perform a branch and (possible) state exchange.
    ///
    /// If the `0th` bit of the `address` is set then the CPU will change to [State::Thumb], otherwise it will switch to
    /// [State::Arm].
    #[inline]
    pub fn branch_and_exchange(cpu: &mut CPU, address: u32, bus: &mut Bus) {
        let to_thumb = address.check_bit(0) as u8;

        // If there is a new state, switch to it
        let new_state = State::from_u8(to_thumb).unwrap();
        cpu.switch_state(new_state, bus);

        // Write new PC value, definitely flushes the pipeline
        cpu.write_reg(PC_REG, address, bus);
    }
}
