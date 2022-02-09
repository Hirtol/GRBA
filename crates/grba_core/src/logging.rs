pub const BIN_TARGET_REGISTER: &str = "register";
pub const BIN_TARGET_DEFAULT: &str = "default";

#[macro_export]
macro_rules! cpu_log {
    ($($arg:tt)*) => {
        #[cfg(feature = "cpu-logging")]
        println!($($arg)*);
    }
}

#[macro_export]
macro_rules! bin_log {
    ($arg:expr) => {
        crate::bin_log!(crate::logging::BIN_TARGET_DEFAULT, $arg);
    };
    ($target:expr, $arg:expr) => {
        #[cfg(feature = "bin-logging")]
        {
            let instr: crate::logging::bin_logging::InstructionSnapshot =
                crate::logging::bin_logging::InstructionSnapshot::from_registers(&$arg);
            crate::logging::bin_logging::log($target, instr.as_ref());
        }
    };
}

pub trait BinaryLogger: Send + Sync {
    fn log_binary(&self, target: &str, data: &[u8]);
}

/// Set the desired logger.
///
/// If the `bin-logging` feature is not enabled this is a no-op.
pub fn set_logger(logger: &'static dyn BinaryLogger) {
    #[cfg(feature = "bin-logging")]
    crate::logging::bin_logging::set_logger(logger);
}

impl BinaryLogger for () {
    fn log_binary(&self, _target: &str, _data: &[u8]) {}
}

#[cfg(feature = "bin-logging")]
pub mod bin_logging {
    use crate::cpu::registers::Registers;
    use crate::logging::BinaryLogger;
    use once_cell::sync::Lazy;

    pub(super) static mut BIN_LOG: Lazy<&dyn BinaryLogger> = Lazy::new(|| &());

    pub fn set_logger(logger: &'static dyn BinaryLogger) {
        // Safety? There isn't any.
        unsafe {
            *BIN_LOG = logger;
        }
    }

    pub fn log(target: &str, data: &[u8]) {
        // Safety? There isn't any.
        unsafe {
            BIN_LOG.log_binary(target, data);
        }
    }

    #[derive(Debug)]
    #[repr(C)]
    pub struct InstructionSnapshot {
        r0: u32,
        r1: u32,
        r2: u32,
        r3: u32,
        r4: u32,
        r5: u32,
        r6: u32,
        r7: u32,
        r8: u32,
        r9: u32,
        r10: u32,
        r11: u32,
        r12: u32,
        r13: u32,
        r14: u32,
        r15: u32,
        cpsr: u32,
        spsr: u32,
    }

    impl AsRef<[u8]> for InstructionSnapshot {
        fn as_ref(&self) -> &[u8] {
            // Safeish, endianness is undefined.
            unsafe { ::std::slice::from_raw_parts(self as *const _ as *const u8, std::mem::size_of::<Self>()) }
        }
    }

    impl InstructionSnapshot {
        pub fn from_registers(reg: &Registers) -> Self {
            InstructionSnapshot {
                r0: reg.general_purpose[0],
                r1: reg.general_purpose[1],
                r2: reg.general_purpose[2],
                r3: reg.general_purpose[3],
                r4: reg.general_purpose[4],
                r5: reg.general_purpose[5],
                r6: reg.general_purpose[6],
                r7: reg.general_purpose[7],
                r8: reg.general_purpose[8],
                r9: reg.general_purpose[9],
                r10: reg.general_purpose[10],
                r11: reg.general_purpose[11],
                r12: reg.general_purpose[12],
                r13: reg.general_purpose[13],
                r14: reg.general_purpose[14],
                r15: reg.general_purpose[15],
                cpsr: reg.cpsr.as_raw(),
                spsr: reg.spsr.as_raw(),
            }
        }
    }
}
