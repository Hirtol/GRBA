use crate::setup;
use grba_core::emulator::cpu::registers::{State, PC_REG, PSR};
use grba_core::emulator::debug::DebugEmulator;
use std::path::Path;

#[test]
pub fn test_fuzz_general() {
    run_fuzz_test("FuzzARM.gba");
}

#[test]
pub fn test_fuzz_arm_any() {
    run_fuzz_test("ARM_Any.gba");
}

#[test]
pub fn test_fuzz_arm_data_processing() {
    run_fuzz_test("ARM_DataProcessing.gba");
}

#[test]
pub fn test_fuzz_thumb_any() {
    run_fuzz_test("THUMB_Any.gba");
}

#[test]
pub fn test_fuzz_thumb_data_processing() {
    run_fuzz_test("THUMB_DataProcessing.gba");
}

/// Run any Fuzz test in `tests/assets/` and print out the failure case if the test fails.
///
/// # Arguments
///
/// * `rom` - The name of the ROM to run.
fn run_fuzz_test(rom: impl AsRef<Path>) {
    let mut emu = setup::get_emu(rom);
    let mut debug_emu = DebugEmulator(&mut emu);

    // Fuzz tests go to 0x0800_00F4 when they're finished
    while debug_emu.cpu().registers.general_purpose[PC_REG] != 0x0800_00F4 {
        debug_emu.0.run_to_vblank();

        // Check if we had a test failure
        if debug_emu.bus().ram.read_board::<u8>(0x0200_0000) != 0 {
            // Give it one more frame to flesh out registers if need be.
            debug_emu.0.run_to_vblank();
            let failure = FuzzArmFailure::from_emu(debug_emu);
            panic!("Failure: {:#010X?}", failure);
        }
    }
}

#[derive(Debug)]
pub struct FuzzArmFailure {
    state: State,
    opcode_or_mult: String,
    initial_regs: InitialRegisters,
    gotten_regs: FailureRegisters,
    expected_regs: FailureRegisters,
}

#[derive(Debug)]
pub struct FailureRegisters {
    r3: u32,
    r4: u32,
    cpsr: PSR,
}

#[derive(Debug)]
pub struct InitialRegisters {
    r0: u32,
    r1: u32,
    r2: u32,
    cpsr: PSR,
}

impl FuzzArmFailure {
    pub fn from_emu(mut emu: DebugEmulator) -> Self {
        let ram = &mut emu.bus().ram;
        let state_val = ram.read_board::<u8>(0x0200_0000) as char;
        let state = if state_val == 'T' { State::Thumb } else { State::Arm };
        let opcode = (4..16)
            .into_iter()
            .map(|i| ram.read_board(0x0200_0000 + i))
            .collect::<Vec<u8>>();
        let opcode_or_mult = String::from_utf8(opcode).unwrap().trim().to_string();

        Self {
            state,
            opcode_or_mult,
            initial_regs: InitialRegisters::from_emu(&mut emu),
            gotten_regs: FailureRegisters::from_emu(&mut emu, false),
            expected_regs: FailureRegisters::from_emu(&mut emu, true),
        }
    }
}

impl FailureRegisters {
    pub fn from_emu(emu: &mut DebugEmulator, get_expected: bool) -> Self {
        let base_address = 0x0200_0020 + if get_expected { 16 } else { 0 };
        let ram = &mut emu.bus().ram;

        Self {
            r3: ram.read_board(base_address),
            r4: ram.read_board(base_address + 4),
            cpsr: ram.read_board::<u32>(base_address + 12).into(),
        }
    }
}

impl InitialRegisters {
    pub fn from_emu(emu: &mut DebugEmulator) -> Self {
        let base_address = 0x0200_0010;
        let ram = &mut emu.bus().ram;

        Self {
            r0: ram.read_board(base_address),
            r1: ram.read_board(base_address + 4),
            r2: ram.read_board(base_address + 8),
            cpsr: ram.read_board::<u32>(base_address + 12).into(),
        }
    }
}
