use crate::utils::check_bit;
use num_traits::FromPrimitive;

/// A `RegisterBank` contains the value of registers for all different modes.
/// Which modes are actually available depends on the register
pub type RegisterBank<const N: usize> = [u32; N];

/// A `SpsrBank` contains the value of the SPSR for all different modes.
pub type SpsrBank = RegisterBank<5>;

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
}

/// Program Status Register, used in the CPSR and SPSR registers.
///
/// Not implemented as raw bitfields due to high-performance requirements.
#[derive(Debug, Clone)]
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
            sign: check_bit(value, 31),
            zero: check_bit(value, 30),
            carry: check_bit(value, 29),
            overflow: check_bit(value, 28),
            irq_disable: check_bit(value, 7),
            fiq_disable: check_bit(value, 6),
            state: State::from_u32(value >> 5 & 1).unwrap(),
            mode: Mode::from_u32(value & 0x1F).unwrap(),
            reserved: value & 0x0FFF_FF00,
        }
    }
}

impl PSR {
    pub fn new() -> PSR {
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

    pub fn from_raw(raw: u32) -> PSR {
        PSR::from(raw)
    }

    pub fn sign(&self) -> bool {
        self.sign
    }

    pub fn zero(&self) -> bool {
        self.zero
    }

    pub fn carry(&self) -> bool {
        self.carry
    }

    pub fn overflow(&self) -> bool {
        self.overflow
    }

    pub fn irq_disable(&self) -> bool {
        self.irq_disable
    }

    pub fn fiq_disable(&self) -> bool {
        self.fiq_disable
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn set_sign(&mut self, value: bool) {
        self.sign = value;
    }

    pub fn set_zero(&mut self, value: bool) {
        self.zero = value;
    }

    pub fn set_carry(&mut self, value: bool) {
        self.carry = value;
    }

    pub fn set_overflow(&mut self, value: bool) {
        self.overflow = value;
    }

    pub fn set_irq_disable(&mut self, value: bool) {
        self.irq_disable = value;
    }

    pub fn set_fiq_disable(&mut self, value: bool) {
        self.fiq_disable = value;
    }

    pub fn set_state(&mut self, value: State) {
        self.state = value;
    }

    pub fn set_mode(&mut self, value: Mode) {
        self.mode = value;
    }

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
}

#[cfg(test)]
mod tests {
    use crate::cpu::registers::{Mode, PSR};

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
