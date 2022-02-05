use crate::bus::Bus;
use crate::cpu::arm::{ArmInstruction, ArmLUT, ArmV4T};
use crate::cpu::registers::{Registers, PC_REG, PSR};
use crate::utils::BitOps;
use registers::{Mode, State};

mod arm;
mod registers;

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
            registers: Registers::new(),
            pipeline: [0; 3],
            arm_lut: arm::create_arm_lut(),
        };

        if skip_bios {
            result.registers.cpsr = registers::PSR::from(0x6000001F);
            result.registers.general_purpose[0] = 0x08000000;
            result.registers.general_purpose[1] = 0x000000EA;
            result.registers.general_purpose[13] = 0x03007F00; // SP

            result.registers.r13_bank[registers::Mode::Supervisor.to_bank_index()] = 0x03007FE0; // SP SVC
            result.registers.r13_bank[registers::Mode::IRQ.to_bank_index()] = 0x03007FA0; // SP IRQ
            result.registers.general_purpose[15] = 0x08000000; // PC
        }

        result.fill_pipeline(bus);

        result
    }

    /// Main stepping function.
    /// Advances the CPU one instruction.
    #[profiling::function]
    pub fn step_instruction(&mut self, bus: &mut Bus) {
        // crate::cpu_log!("Executing instruction: {:#X}", self.pipeline[0]);
        crate::cpu_log!("Registers: {:X?}", self.registers);

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

        self.advance_pipeline(bus);
    }

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

    /// Clear the entire pipeline, and immediately refills it afterwards.
    fn clear_pipeline(&mut self, bus: &mut Bus) {
        self.pipeline = [0; 3];
        self.fill_pipeline(bus);
    }

    fn execute_arm(&mut self, instruction: ArmInstruction, bus: &mut Bus) {
        if !ArmV4T::condition_holds(self, instruction) {
            return;
        }

        let lut_index = (((instruction.get_bits(20, 27)) << 4) | instruction.get_bits(4, 7)) as usize;

        crate::cpu_log!("Executing LUT: {:#b} - Raw: {:#X}", lut_index, self.pipeline[0]);
        self.arm_lut[lut_index](self, instruction, bus);
    }

    fn execute_thumb(&mut self, bus: &mut Bus, opcode: u16) {
        todo!()
    }

    fn raise_exception(&mut self, exception: Exception, bus: &mut Bus) {
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
    pub fn switch_state(&mut self, new_state: State, bus: &mut Bus) {
        // Switch to a new state, and refill the pipeline
        if self.state() != new_state {
            self.registers.cpsr.set_state(new_state);
            self.fill_pipeline(bus);
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
            // Why we subtract one instruction I have no clue, when testing we were always one instruction to far
            // after a branch, so here it is.
            let to_write = match self.state() {
                State::Arm => (value - 4) & 0xFFFF_FFFC,
                State::Thumb => (value - 2) & 0xFFFF_FFFE,
            };

            self.registers.write_reg(PC_REG, to_write);

            self.fill_pipeline(bus);
        }
    }

    #[inline(always)]
    fn state(&self) -> State {
        self.registers.cpsr.state()
    }
}

fn log_cpu_state(cpu: &CPU) {
    println!("{:X?}", cpu.registers);
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
