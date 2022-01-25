/// The main emulator struct
#[derive(Debug, Clone)]
pub struct GBAEmulator {
    pub(crate) cpu: u64,
    pub(crate) mmu: u64,
    pub(crate) scheduler: u64,
}

impl GBAEmulator {
    /// Run the emulator until it has reached Vblank
    #[profiling::function]
    pub fn run_to_vblank(&mut self) {
        while !self.emulate_cycle() {}
        profiling::finish_frame!();
    }

    pub fn emulate_cycle(&mut self) -> bool {
        true
    }
}
