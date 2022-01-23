/// The main emulator struct
#[derive(Debug, Clone)]
pub struct GBAEmulator {
    pub(crate) cpu: u64,
    pub(crate) mmu: u64,
    pub(crate) scheduler: u64,
}
