use crate::utils::BitOps;
use num_traits::FromPrimitive;

/// Index of PC register
pub const PC_REG: usize = 15;
/// Index of the link register
pub const LINK_REG: usize = 14;
/// Index of the stack pointer register
pub const SP_REG: usize = 13;

/// A `RegisterBank` contains the value of registers for all different modes.
/// Which modes are actually available depends on the register
pub type RegisterBank<const N: usize> = [u32; N];

/// A `SpsrBank` contains the value of the SPSR for all different modes.
pub type SpsrBank = [PSR; 5];

/// Contains all CPU registers.
/// More Info: [Here](https://problemkaputt.de/gbatek.htm#armcpuregisterset)
#[derive(Debug)]
pub struct Registers {
    /// R0-R12 Registers (General Purpose Registers).
    /// These thirteen registers may be used for whatever general purposes.
    /// Basically, each is having same functionality and performance, ie.
    /// there is no 'fast accumulator' for arithmetic operations, and no 'special pointer register' for memory addressing.
    ///
    /// However, in THUMB mode only R0-R7 (Lo registers) may be accessed freely,
    /// while R8-R12 and up (Hi registers) can be accessed only by some instructions.
    ///
    pub general_purpose: [u32; 16],

    /// CPU condition codes and control bits
    pub cpsr: PSR,
    /// Old CPSR prior to the current exception-mode being called.
    pub spsr: PSR,
    pub spsr_bank: SpsrBank,

    // Storage banks for the different modes the CPU can be in
    pub r8_bank: RegisterBank<2>,
    pub r9_bank: RegisterBank<2>,
    pub r10_bank: RegisterBank<2>,
    pub r11_bank: RegisterBank<2>,
    pub r12_bank: RegisterBank<2>,
    pub r13_bank: RegisterBank<6>,
    pub r14_bank: RegisterBank<6>,
}

impl Default for Registers {
    fn default() -> Self {
        Registers {
            general_purpose: [0; 16],
            cpsr: PSR::from_raw(0x000000DF),
            spsr: PSR::from_raw(0x000000DF),
            spsr_bank: [PSR::default(); 5],
            r8_bank: [0; 2],
            r9_bank: [0; 2],
            r10_bank: [0; 2],
            r11_bank: [0; 2],
            r12_bank: [0; 2],
            r13_bank: [0; 6],
            r14_bank: [0; 6],
        }
    }
}

impl Registers {
    #[inline(always)]
    pub fn pc(&self) -> u32 {
        self.general_purpose[PC_REG]
    }

    /// Swap the register banks. Saving the current registers in the `from_mode` bank, and loading the `to_mode` bank.
    /// Does *not* switch the mode in the CPSR, and in fact leaves the CPSR as it was.
    ///
    /// # Returns
    ///
    /// * `true` if the `from_mode` and `to_mode` are different (swapped).
    /// * `false` if the `from_mode` and `to_mode` are the same (early return, no swap).
    #[inline]
    pub fn swap_register_banks(&mut self, from_mode: Mode, to_mode: Mode) -> bool {
        if from_mode == to_mode {
            return false;
        }

        let from_bank_idx = from_mode.to_bank_index();
        let to_bank_idx = to_mode.to_bank_index();

        // Save the unique banks
        if from_mode == Mode::FIQ {
            // Save current FIQ registers to FIQ bank
            let fiq_bank = from_bank_idx;
            self.r8_bank[fiq_bank] = self.general_purpose[8];
            self.r9_bank[fiq_bank] = self.general_purpose[9];
            self.r10_bank[fiq_bank] = self.general_purpose[10];
            self.r11_bank[fiq_bank] = self.general_purpose[11];
            self.r12_bank[fiq_bank] = self.general_purpose[12];
        } else {
            // All other modes share a register bank
            let user_bank = Mode::User.to_bank_index();
            self.r8_bank[user_bank] = self.general_purpose[8];
            self.r9_bank[user_bank] = self.general_purpose[9];
            self.r10_bank[user_bank] = self.general_purpose[10];
            self.r11_bank[user_bank] = self.general_purpose[11];
            self.r12_bank[user_bank] = self.general_purpose[12];
        }

        self.r13_bank[from_bank_idx] = self.general_purpose[13];
        self.r14_bank[from_bank_idx] = self.general_purpose[14];

        // Now move all banked registers of the new mode to the current registers
        if to_mode == Mode::FIQ {
            let fiq_bank = to_bank_idx;
            self.general_purpose[8] = self.r8_bank[fiq_bank];
            self.general_purpose[9] = self.r9_bank[fiq_bank];
            self.general_purpose[10] = self.r10_bank[fiq_bank];
            self.general_purpose[11] = self.r11_bank[fiq_bank];
            self.general_purpose[12] = self.r12_bank[fiq_bank];
        } else {
            let user_bank = Mode::User.to_bank_index();
            self.general_purpose[8] = self.r8_bank[user_bank];
            self.general_purpose[9] = self.r9_bank[user_bank];
            self.general_purpose[10] = self.r10_bank[user_bank];
            self.general_purpose[11] = self.r11_bank[user_bank];
            self.general_purpose[12] = self.r12_bank[user_bank];
        }

        self.general_purpose[13] = self.r13_bank[to_bank_idx];
        self.general_purpose[14] = self.r14_bank[to_bank_idx];

        true
    }

    #[inline(always)]
    pub(crate) fn read_reg(&self, reg: usize) -> u32 {
        self.general_purpose[reg]
    }

    /// Write to a register.
    ///
    /// Note that this can not be used by anyone but the [crate::cpu::CPU] itself, as this does not update the pipeline
    /// if [PC_REG] is written to.
    #[inline(always)]
    pub(crate) fn write_reg(&mut self, reg: usize, value: u32) {
        self.general_purpose[reg] = value;
    }

    #[inline(always)]
    pub(crate) fn advance_pc(&mut self) {
        match self.cpsr.state() {
            State::Arm => {
                self.general_purpose[PC_REG] += 4;
            }
            State::Thumb => {
                self.general_purpose[PC_REG] += 2;
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, num_derive::FromPrimitive)]
pub enum State {
    /// 32 Bit opcodes.
    Arm = 0b0,
    /// 16 Bit opcodes
    Thumb = 0b1,
}

/// The mode the CPU can find itself in.
/// Triggered by different exceptions.
#[derive(Debug, Eq, PartialEq, Copy, Clone, num_derive::FromPrimitive)]
pub enum Mode {
    User = 0b10000,
    FIQ = 0b10001,
    IRQ = 0b10010,
    Supervisor = 0b10011,
    Abort = 0b10111,
    Undefined = 0b11011,
    System = 0b11111,
}

impl Mode {
    /// Converts the current mode to an index for a [RegisterBank]
    pub const fn to_bank_index(self) -> usize {
        match self {
            Mode::User | Mode::System => 0,
            Mode::FIQ => 1,
            Mode::IRQ => 4,
            Mode::Supervisor => 2,
            Mode::Abort => 3,
            Mode::Undefined => 5,
        }
    }

    /// Converts the mode to an index for a [SpsrBank]
    pub const fn to_spsr_index(self) -> usize {
        match self {
            Mode::User | Mode::System => panic!("Cannot get SPSR for User/System mode"),
            Mode::FIQ => 0,
            Mode::IRQ => 3,
            Mode::Supervisor => 1,
            Mode::Abort => 2,
            Mode::Undefined => 4,
        }
    }

    pub const fn has_spsr(self) -> bool {
        !matches!(self, Mode::User | Mode::System)
    }
}

/// Program Status Register, used in the CPSR and SPSR registers.
///
/// Not implemented as raw bitfields due to high-performance requirements.
#[derive(Debug, Clone, Copy)]
pub struct PSR {
    sign: bool,
    zero: bool,
    carry: bool,
    overflow: bool,
    irq_disable: bool,
    fiq_disable: bool,
    state: State,
    mode: Mode,

    reserved: u32,
}

impl From<u32> for PSR {
    fn from(value: u32) -> Self {
        PSR {
            sign: value.check_bit(31),
            zero: value.check_bit(30),
            carry: value.check_bit(29),
            overflow: value.check_bit(28),
            irq_disable: value.check_bit(7),
            fiq_disable: value.check_bit(6),
            state: State::from_u32(value >> 5 & 1).unwrap(),
            mode: Mode::from_u32(value & 0x1F).unwrap(),
            reserved: value & 0x0FFF_FF00,
        }
    }
}

impl Default for PSR {
    fn default() -> Self {
        PSR {
            sign: false,
            zero: false,
            carry: false,
            overflow: false,
            irq_disable: false,
            fiq_disable: false,
            state: State::Arm,
            mode: Mode::User,
            reserved: 0,
        }
    }
}

impl PSR {
    #[inline(always)]
    pub fn from_raw(raw: u32) -> PSR {
        PSR::from(raw)
    }

    #[inline(always)]
    pub fn sign(&self) -> bool {
        self.sign
    }

    #[inline(always)]
    pub fn zero(&self) -> bool {
        self.zero
    }

    #[inline(always)]
    pub fn carry(&self) -> bool {
        self.carry
    }

    #[inline(always)]
    pub fn overflow(&self) -> bool {
        self.overflow
    }

    #[inline(always)]
    pub fn irq_disable(&self) -> bool {
        self.irq_disable
    }

    #[inline(always)]
    pub fn fiq_disable(&self) -> bool {
        self.fiq_disable
    }

    #[inline(always)]
    pub fn state(&self) -> State {
        self.state
    }

    #[inline(always)]
    pub fn mode(&self) -> Mode {
        self.mode
    }

    #[inline(always)]
    pub fn set_sign(&mut self, value: bool) {
        self.sign = value;
    }

    #[inline(always)]
    pub fn set_zero(&mut self, value: bool) {
        self.zero = value;
    }

    #[inline(always)]
    pub fn set_carry(&mut self, value: bool) {
        self.carry = value;
    }

    #[inline(always)]
    pub fn set_overflow(&mut self, value: bool) {
        self.overflow = value;
    }

    #[inline(always)]
    pub fn set_irq_disable(&mut self, value: bool) {
        self.irq_disable = value;
    }

    #[inline(always)]
    pub fn set_fiq_disable(&mut self, value: bool) {
        self.fiq_disable = value;
    }

    #[inline(always)]
    pub fn set_state(&mut self, value: State) {
        self.state = value;
    }

    #[inline(always)]
    pub fn set_mode(&mut self, value: Mode) {
        self.mode = value;
    }

    /// Pack the contents of the PSR into a single 32-bit value.
    #[inline]
    pub fn as_raw(&self) -> u32 {
        let mut result = self.reserved;
        result |= (self.sign as u32) << 31;
        result |= (self.zero as u32) << 30;
        result |= (self.carry as u32) << 29;
        result |= (self.overflow as u32) << 28;

        result |= (self.irq_disable as u32) << 7;
        result |= (self.fiq_disable as u32) << 6;
        result |= (self.state as u32) << 5;
        result |= self.mode as u32;
        result
    }

    /// Updates only the control flags of the PSR.
    /// The provided `value` will therefore only have the most significant `4` bits examined.
    pub fn update_control_flags(&mut self, value: u32) {
        self.sign = value.check_bit(31);
        self.zero = value.check_bit(30);
        self.carry = value.check_bit(29);
        self.overflow = value.check_bit(28);
    }
}

#[cfg(test)]
mod tests {
    use crate::emulator::cpu::registers::{Mode, PSR};

    #[test]
    fn psr_test() {
        let value = 0b1101_0000_0000_0000_0000_0000_1011_0000;
        let cpsr = PSR::from(value);
        println!("{:?}", cpsr);

        assert!(cpsr.sign());
        assert!(cpsr.zero());
        assert!(!cpsr.carry());
        assert!(cpsr.overflow());

        assert!(cpsr.irq_disable());
        assert!(!cpsr.fiq_disable());
        assert_eq!(cpsr.mode(), Mode::User);
        assert_eq!(cpsr.state(), super::State::Thumb);
        assert_eq!(cpsr.as_raw(), value);
    }
}
