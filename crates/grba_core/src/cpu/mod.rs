use crate::bus::Bus;
use crate::cpu::arm::{ArmInstruction, ArmLUT};
use crate::cpu::registers::Registers;
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
        crate::cpu_log!("Executing instruction: {:#X}", self.pipeline[0]);
        crate::cpu_log!("Registers: {:?}", self.registers);

        match self.state() {
            State::Arm => {
                self.execute_arm(self.pipeline[0], bus);
            }
            State::Thumb => {
                self.execute_thumb(bus, self.pipeline[0] as u16);
            }
        }

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

    /// Clear the entire pipeline.
    fn clear_pipeline(&mut self) {
        self.pipeline = [0; 3];
    }

    fn execute_arm(&mut self, instruction: ArmInstruction, bus: &mut Bus) {
        if !self.condition_holds(instruction) {
            return;
        }

        let lut_index = (((instruction >> 4) & 0xF) | ((instruction & 0x0FF0_0000) >> 16)) as usize;
        self.arm_lut[lut_index](self, instruction, bus);
    }

    fn execute_thumb(&mut self, bus: &mut Bus, opcode: u16) {}

    fn raise_exception(&mut self, exception: Exceptions, bus: &mut Bus) {
        todo!()
    }

    /// Read from a general purpose register.
    /// `reg` should be in the range 0..16
    #[inline(always)]
    fn read_reg(&self, reg: usize) -> u32 {
        self.registers.general_purpose[reg]
    }

    /// Write to a general purpose register.
    /// `reg` should be in the range 0..16
    #[inline(always)]
    fn write_reg(&mut self, reg: usize, value: u32) {
        self.registers.general_purpose[reg] = value;
    }

    #[inline(always)]
    fn state(&self) -> State {
        self.registers.cpsr.state()
    }
}

fn log_cpu_state(cpu: &CPU) {
    println!("{:?}", cpu.registers);
}

#[derive(Debug)]
pub enum Exceptions {
    SoftwareInterrupt,
    UndefinedInstruction,
    PrefetchAbort,
    FastInterrupt,
    Interrupt,
    DataAbort,
    Reset,
}
