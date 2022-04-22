use crate::emulator::bus::Bus;
use crate::emulator::cpu::arm::{ArmInstruction, ArmLUT, ArmV4};
use crate::emulator::cpu::registers::{Registers, LINK_REG, PC_REG};
use crate::emulator::cpu::thumb::{ThumbInstruction, ThumbLUT};
use crate::utils::BitOps;
use registers::{Mode, State};

mod arm;
mod common;
pub mod registers;
mod thumb;

//TODO Timings:
// * Gamepak fetching and prefetching?
// * Instruction timings

pub struct CPU {
    pub registers: Registers,
    /// GBA has a pipeline of execute-decode-fetch.
    pub pipeline: [u32; 3],
    pub arm_lut: ArmLUT,
    pub thumb_lut: ThumbLUT,
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
            thumb_lut: thumb::create_thumb_lut(),
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
    #[inline(always)]
    pub fn step_instruction(&mut self, bus: &mut Bus) {
        // We immediately advance the pipeline once to recover from pipeline flush (which only partly fills the pipeline)
        self.advance_pipeline(bus);

        crate::cpu_log!("Registers: {:X?}", self.registers);
        #[cfg(feature = "bin-logging")]
        log_cpu_state(self);

        match self.state() {
            State::Arm => {
                self.execute_arm(bus, self.pipeline[0]);
            }
            State::Thumb => {
                self.execute_thumb(bus, self.pipeline[0] as ThumbInstruction);
            }
        }
    }

    // Sure hope this gets inlined to prevent excessive `match self.state() {}` calls >.>
    #[inline(always)]
    fn advance_pipeline(&mut self, bus: &mut Bus) {
        self.pipeline[0] = self.pipeline[1];
        self.pipeline[1] = self.pipeline[2];

        // Advance the PC depending on state
        self.registers.advance_pc();

        // Prefetch the next instruction
        self.pipeline[2] = match self.state() {
            State::Arm => bus.read_32(self.registers.pc(), self),
            State::Thumb => bus.read_16(self.registers.pc(), self) as u32,
        };
    }

    /// Clear the entire pipeline, and partly refills it afterwards.
    ///
    /// This is a partial refill to account for us immediately incrementing the PC when we next execute an instruction.
    fn flush_pipeline(&mut self, bus: &mut Bus) {
        self.pipeline[0] = 0;

        match self.state() {
            State::Arm => {
                self.pipeline[1] = bus.read_32(self.registers.pc(), self);
                self.registers.advance_pc();
                self.pipeline[2] = bus.read_32(self.registers.pc(), self);
            }
            State::Thumb => {
                self.pipeline[1] = bus.read_16(self.registers.pc(), self) as u32;
                self.registers.advance_pc();
                self.pipeline[2] = bus.read_16(self.registers.pc(), self) as u32;
            }
        }
    }

    #[profiling::function]
    #[inline(always)]
    fn execute_arm(&mut self, bus: &mut Bus, instruction: ArmInstruction) {
        if !ArmV4::condition_holds(self, instruction) {
            return;
        }

        let lut_index = (((instruction.get_bits(20, 27)) << 4) | instruction.get_bits(4, 7)) as usize;

        crate::cpu_log!("Executing Arm LUT: {:#b} - Raw: {:#X}", lut_index, instruction);
        self.arm_lut[lut_index](self, instruction, bus);
    }

    #[profiling::function]
    #[inline(always)]
    fn execute_thumb(&mut self, bus: &mut Bus, instruction: ThumbInstruction) {
        let lut_index = instruction.get_bits(8, 15) as usize;

        crate::cpu_log!("Executing Thumb LUT: {:#b} - Raw: {:#X}", lut_index, instruction);
        self.thumb_lut[lut_index](self, instruction, bus);
    }

    fn switch_mode(&mut self, new_mode: registers::Mode, _bus: &mut Bus) {
        let old_mode = self.registers.cpsr.mode();

        // Try to swap, if we're already in the same mode this'll fail.
        if !self.registers.swap_register_banks(old_mode, new_mode, true) {
            return;
        }

        self.registers.cpsr.set_mode(new_mode);
    }

    /// Switches between ARM and Thumb mode.
    pub fn switch_state(&mut self, new_state: State, _bus: &mut Bus) {
        // Switch to a new state
        crate::cpu_log!("Switching CPU state to {:?}", new_state);
        if self.state() != new_state {
            self.registers.cpsr.set_state(new_state);
            // TODO: Do we need to flush pipeline here? At the very least probably need to align PC to new state?
        }
    }

    fn raise_exception(&mut self, bus: &mut Bus, exception: Exception) {
        // SoftwareInterrupt and IRQ are the only exceptions that can be raised (besides UndefinedInstruction) in the GBA
        const SOFTWARE_INTERRUPT_ADDR: u32 = 0x00000008;
        const IRQ_ADDR: u32 = 0x00000018;

        let (pipeline_subtraction, jump_addr, new_mode) = match exception {
            Exception::SoftwareInterrupt => {
                crate::cpu_log!("Raising Software Interrupt");
                let pipeline_subtraction = match self.state() {
                    State::Arm => 4,
                    State::Thumb => 2,
                };

                (pipeline_subtraction, SOFTWARE_INTERRUPT_ADDR, Mode::Supervisor)
            }
            Exception::Interrupt => {
                crate::cpu_log!("Raising IRQ");
                // In this case users should return to the PC before this interrupt would be executed using the SUBS, r14, #4 instruction.
                // For Thumb we therefore need to not subtract anything, and for ARM only 4.
                let pipeline_subtraction = match self.state() {
                    State::Arm => 4,
                    State::Thumb => 0,
                };

                (pipeline_subtraction, IRQ_ADDR, Mode::IRQ)
            }
            _ => todo!(
                "Other exceptions aren't used in the GBA? {:?} - {:?}",
                exception,
                bus.scheduler.current_time
            ),
        };

        let link_reg_value = self.read_reg(PC_REG) - pipeline_subtraction;
        let old_cpsr = self.registers.cpsr;
        // Change CPU state to ARM (if not already)
        self.switch_state(State::Arm, bus);
        // Enter supervisor mode
        self.switch_mode(new_mode, bus);
        // Set the link register to the next instruction
        self.write_reg(LINK_REG, link_reg_value, bus);
        // Jump to the exception handler
        self.write_reg(PC_REG, jump_addr, bus);
        // Disable any further interrupts
        self.registers.cpsr.set_irq_disable(true);
        // Preserve our old cpsr
        self.registers.spsr = old_cpsr;
    }

    pub fn poll_interrupts(&mut self, bus: &mut Bus) {
        let ie: u16 = bus.interrupts.enable.into();
        let iflags: u16 = bus.interrupts.flags.into();

        // Check if an interrupt in `iflags` has been raised, and that it has also been enabled in `ie`
        if ie & iflags != 0 {
            // Take out of HALT mode, then check if interrupts are enabled
            bus.system_control.is_halted = false;

            // If master interrupts are disabled, then just don't bother.
            if self.registers.cpsr.irq_disable() || !bus.interrupts.master_enable.interrupt_enable() {
                return;
            }

            // Otherwise, raise the IRQ exception
            self.raise_exception(bus, Exception::Interrupt);
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
    /// Unused in GBA
    PrefetchAbort,
    /// Unused in GBA, only IRQ is used
    FastInterrupt,
    Interrupt,
    /// Unused in GBA
    DataAbort,
    Reset,
}
