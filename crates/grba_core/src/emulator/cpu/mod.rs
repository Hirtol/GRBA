use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmLUT, ArmV4};
use crate::emulator::cpu::registers::{Registers, PC_REG};
use crate::utils::BitOps;
use registers::{Mode, State};

mod arm;
pub mod registers;
mod thumb;

//TODO Timings:
// * Gamepak fetching and prefetching?
// * Instruction timings

pub struct CPU {
    registers: Registers,
    /// GBA has a pipeline of execute-decode-fetch.
    pipeline: [u32; 3],
    arm_lut: ArmLUT,
}

impl CPU {
    /// Create a new CPU.
    ///
    /// # Arguments
    ///
    /// * `skip_bios` - Whether to skip the BIOS. If skipped will initialise appropriate registers
    pub fn new(skip_bios: bool, bus: &mut Bus) -> CPU {
        let mut result = CPU {
            registers: Registers::default(),
            pipeline: [0; 3],
            arm_lut: arm::create_arm_lut(),
        };

        if skip_bios {
            // Temporarily commented out for log diffing.
            // result.registers.cpsr = registers::PSR::from(0x6000001F);
            // result.registers.general_purpose[0] = 0x08000000;
            // result.registers.general_purpose[1] = 0x000000EA;
            result.registers.general_purpose[13] = 0x03007F00; // SP

            result.registers.r13_bank[registers::Mode::Supervisor.to_bank_index()] = 0x03007FE0; // SP SVC
            result.registers.r13_bank[registers::Mode::IRQ.to_bank_index()] = 0x03007FA0; // SP IRQ
            result.registers.general_purpose[15] = 0x08000000; // PC
        }

        result.flush_pipeline(bus);

        result
    }

    /// Main stepping function.
    /// Advances the CPU one instruction.
    #[profiling::function]
    pub fn step_instruction(&mut self, bus: &mut Bus) {
        // We immediately advance the pipeline once to recover from pipeline flush (which only partly fills the pipeline)
        self.advance_pipeline(bus);

        crate::cpu_log!("Registers: {:X?}", self.registers);
        #[cfg(feature = "bin-logging")]
        log_cpu_state(self);

        match self.state() {
            State::Arm => {
                self.execute_arm(self.pipeline[0], bus);
            }
            State::Thumb => {
                self.execute_thumb(bus, self.pipeline[0] as u16);
            }
        }
        // Very basic cycle counting to get things going. In the future ought to count cycles properly.
        //TODO: Instruction timing
        bus.scheduler.add_time(1);
    }

    // Sure hope this gets inlined to prevent excessive `match self.state() {}` calls >.>
    #[inline]
    fn advance_pipeline(&mut self, bus: &mut Bus) {
        self.pipeline[0] = self.pipeline[1];
        self.pipeline[1] = self.pipeline[2];

        // Advance the PC depending on state
        self.registers.advance_pc();

        // Prefetch the next instruction
        self.pipeline[2] = match self.state() {
            State::Arm => bus.read_32(self.registers.pc()),
            State::Thumb => bus.read_16(self.registers.pc()) as u32,
        };
    }

    /// Immediately fill the entire pipeline with instructions, starting at `pc`.
    fn fill_pipeline(&mut self, bus: &mut Bus) {
        match self.state() {
            State::Arm => {
                self.pipeline[0] = bus.read_32(self.registers.pc());
                self.registers.advance_pc();
                self.pipeline[1] = bus.read_32(self.registers.pc());
                self.registers.advance_pc();
                self.pipeline[2] = bus.read_32(self.registers.pc());
            }
            State::Thumb => {
                self.pipeline[0] = bus.read_16(self.registers.pc()) as u32;
                self.registers.advance_pc();
                self.pipeline[1] = bus.read_16(self.registers.pc()) as u32;
                self.registers.advance_pc();
                self.pipeline[2] = bus.read_16(self.registers.pc()) as u32;
            }
        }
    }

    /// Clear the entire pipeline, and partly refills it afterwards.
    ///
    /// This is a partial refill to account for us immediately incrementing the PC when we next execute an instruction.
    fn flush_pipeline(&mut self, bus: &mut Bus) {
        self.pipeline[0] = 0;

        match self.state() {
            State::Arm => {
                self.pipeline[1] = bus.read_32(self.registers.pc());
                self.registers.advance_pc();
                self.pipeline[2] = bus.read_32(self.registers.pc());
            }
            State::Thumb => {
                self.pipeline[1] = bus.read_16(self.registers.pc()) as u32;
                self.registers.advance_pc();
                self.pipeline[2] = bus.read_16(self.registers.pc()) as u32;
            }
        }
    }

    fn execute_arm(&mut self, instruction: ArmInstruction, bus: &mut Bus) {
        if !ArmV4::condition_holds(self, instruction) {
            return;
        }

        let lut_index = (((instruction.get_bits(20, 27)) << 4) | instruction.get_bits(4, 7)) as usize;

        crate::cpu_log!("Executing LUT: {:#b} - Raw: {:#X}", lut_index, instruction);
        self.arm_lut[lut_index](self, instruction, bus);
    }

    fn execute_thumb(&mut self, _bus: &mut Bus, _opcode: u16) {
        todo!()
    }

    fn raise_exception(&mut self, exception: Exception, _bus: &mut Bus) {
        todo!("{:?}", exception);
    }

    fn switch_mode(&mut self, new_mode: registers::Mode) {
        let old_mode = self.registers.cpsr.mode();

        // Try to swap, if we're already in the same mode this'll fail.
        if !self.registers.swap_register_banks(old_mode, new_mode) {
            return;
        }

        self.registers.cpsr.set_mode(new_mode);

        match old_mode {
            Mode::User | Mode::System => {}
            _ => {
                self.registers.spsr_bank[old_mode.to_spsr_index()] = self.registers.spsr;
            }
        }

        match new_mode {
            Mode::User | Mode::System => {
                // Not sure if we should re-create the PSR.
                // self.registers.spsr = PSR::new();
            }
            _ => {
                self.registers.spsr = self.registers.spsr_bank[old_mode.to_spsr_index()];
            }
        }
    }

    /// Switches between ARM and Thumb mode.
    pub fn switch_state(&mut self, new_state: State, _bus: &mut Bus) {
        // Switch to a new state
        if self.state() != new_state {
            self.registers.cpsr.set_state(new_state);
            // TODO: Do we need to flush pipeline here? At the very least probably need to align PC to new state?
        }
    }

    /// Read from a general purpose register.
    /// `reg` should be in the range 0..16
    #[inline(always)]
    fn read_reg(&self, reg: usize) -> u32 {
        self.registers.read_reg(reg)
    }

    /// Write to a general purpose register.
    /// `reg` should be in the range 0..16
    #[inline(always)]
    fn write_reg(&mut self, reg: usize, value: u32, bus: &mut Bus) {
        if reg != PC_REG {
            self.registers.write_reg(reg, value)
        } else {
            // Upon writes to PC we need to flush our instruction cache, and also block out the lower bits.
            let to_write = match self.state() {
                State::Arm => value & 0xFFFF_FFFC,
                State::Thumb => value & 0xFFFF_FFFE,
            };

            self.registers.write_reg(PC_REG, to_write);

            self.flush_pipeline(bus);
        }
    }

    #[inline(always)]
    fn state(&self) -> State {
        self.registers.cpsr.state()
    }
}

#[inline(always)]
fn log_cpu_state(cpu: &CPU) {
    let frame = crate::logging::InstructionFrame {
        registers: crate::logging::InstructionSnapshot::from_registers(&cpu.registers),
        instruction: cpu.pipeline[0],
    };

    crate::bin_log!(crate::logging::BIN_TARGET_FRAME, frame.as_ref());
}

#[derive(Debug)]
pub enum Exception {
    SoftwareInterrupt,
    UndefinedInstruction,
    PrefetchAbort,
    FastInterrupt,
    Interrupt,
    DataAbort,
    Reset,
}
