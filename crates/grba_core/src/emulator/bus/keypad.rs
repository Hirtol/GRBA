use crate::emulator::bus::interrupts::{InterruptManager, Interrupts};
use crate::emulator::MemoryAddress;
use crate::scheduler::Scheduler;
use crate::InputKeys;

pub const KEYSTATUS_START: MemoryAddress = 0x0400_0130;
pub const KEYSTATUS_END: MemoryAddress = 0x0400_0131;
pub const KEYINTERRUPT_START: MemoryAddress = 0x0400_0132;
pub const KEYINTERRUPT_END: MemoryAddress = 0x0400_0133;

#[derive(Default)]
pub struct Keypad {
    pub status: KeypadStatus,
    pub interrupt_control: KeypadInterruptControl,
}

impl Keypad {
    pub fn button_changed(
        &mut self,
        button: InputKeys,
        pressed: bool,
        scheduler: &mut Scheduler,
        interrupt: &mut InterruptManager,
    ) {
        let is_released = !pressed;

        match button {
            InputKeys::Start => {
                self.status.set_start(is_released);
            }
            InputKeys::Select => {
                self.status.set_select(is_released);
            }
            InputKeys::A => {
                self.status.set_button_a(is_released);
            }
            InputKeys::B => {
                self.status.set_button_b(is_released);
            }
            InputKeys::Up => {
                self.status.set_up(is_released);
            }
            InputKeys::Down => {
                self.status.set_down(is_released);
            }
            InputKeys::Left => {
                self.status.set_left(is_released);
            }
            InputKeys::Right => {
                self.status.set_right(is_released);
            }
            InputKeys::ShoulderLeft => {
                self.status.set_shoulder_left(is_released);
            }
            InputKeys::ShoulderRight => {
                self.status.set_shoulder_right(is_released);
            }
        }

        if self.interrupt_control.button_irq_enable() {
            let irq_buttons = u16::from_le_bytes(self.interrupt_control.to_le_bytes()) & 0x3FF;
            // We invert it to get it such that the bit is set if the button is pressed
            let buttons = (!u16::from_le_bytes(self.status.to_le_bytes())) & 0x3FF;

            if self.interrupt_control.button_irq_condition() {
                // Logical and, interrupt requested if ALL of the desired buttons are pressed
                if (buttons & irq_buttons) == irq_buttons {
                    interrupt.request_interrupt(Interrupts::Keypad, scheduler);
                }
            } else if buttons & irq_buttons != 0 {
                // Logical or, interrupt requested if ANY of the desired buttons is pressed
                interrupt.request_interrupt(Interrupts::Keypad, scheduler);
            }
        }
    }
}

/// Displays the status of keypad buttons.
///
/// Read only.
#[modular_bitfield::bitfield(bits = 16)]
#[repr(u16)]
#[derive(Debug, Copy, Clone)]
pub struct KeypadStatus {
    button_a: bool,
    button_b: bool,
    select: bool,
    start: bool,
    right: bool,
    left: bool,
    up: bool,
    down: bool,
    shoulder_right: bool,
    shoulder_left: bool,
    #[skip]
    unused: modular_bitfield::prelude::B6,
}

impl Default for KeypadStatus {
    fn default() -> Self {
        // Button bit: 1 == released, 0 == pressed
        0x03FF.into()
    }
}

#[modular_bitfield::bitfield(bits = 16)]
#[repr(u16)]
#[derive(Debug, Copy, Clone, Default)]
pub struct KeypadInterruptControl {
    button_a: bool,
    button_b: bool,
    select: bool,
    start: bool,
    right: bool,
    left: bool,
    up: bool,
    down: bool,
    shoulder_right: bool,
    shoulder_left: bool,
    #[skip]
    unused: modular_bitfield::prelude::B4,
    button_irq_enable: bool,
    /// (0=Logical OR, 1=Logical AND)
    /// In logical OR mode, an interrupt is requested when at least one of the selected buttons is pressed.
    /// In logical AND mode, an interrupt is requested when ALL of the selected buttons are pressed.
    button_irq_condition: bool,
}
