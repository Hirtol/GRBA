/// Refers to an *absolute* memory address.
/// Therefore any component which takes this as an incoming type *must* pre-process the value to turn it into an address
/// relative to itself.
pub(crate) type MemoryAddress = u32;

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
