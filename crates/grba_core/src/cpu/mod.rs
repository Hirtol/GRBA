use crate::cpu::registers::Registers;
use registers::{Mode, State};

mod registers;

#[derive(Debug)]
pub struct CPU {
    state: State,
    mode: Mode,
    registers: Registers,
}
