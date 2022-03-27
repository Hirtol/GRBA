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
                self.status.buttons().set_start(is_released);
            }
            InputKeys::Select => {
                self.status.buttons().set_select(is_released);
            }
            InputKeys::A => {
                self.status.buttons().set_button_a(is_released);
            }
            InputKeys::B => {
                self.status.buttons().set_button_b(is_released);
            }
            InputKeys::Up => {
                self.status.buttons().set_up(is_released);
            }
            InputKeys::Down => {
                self.status.buttons().set_down(is_released);
            }
            InputKeys::Left => {
                self.status.buttons().set_left(is_released);
            }
            InputKeys::Right => {
                self.status.buttons().set_right(is_released);
            }
            InputKeys::ShoulderLeft => {
                self.status.buttons().set_shoulder_left(is_released);
            }
            InputKeys::ShoulderRight => {
                self.status.buttons().set_shoulder_right(is_released);
            }
        }

        if self.interrupt_control.button_irq_enable() {
            let irq_buttons = u16::from_le_bytes(self.interrupt_control.buttons().to_le_bytes());
            // We invert it to get it such that the bit is set if the button is pressed
            let buttons = !u16::from_le_bytes(self.status.buttons().to_le_bytes());

            if self.interrupt_control.button_irq_condition() {
                // Logical and, interrupt requested if ALL of the desired buttons are pressed
                if buttons == irq_buttons {
                    interrupt.request_interrupt(Interrupts::Keypad, scheduler);
                }
            } else if buttons & irq_buttons != 0 {
                // Logical or, interrupt requested if ANY of the desired buttons is pressed
                interrupt.request_interrupt(Interrupts::Keypad, scheduler);
            }
        }
    }
}

#[modular_bitfield::bitfield(bits = 10)]
#[derive(Debug, BitfieldSpecifier, Default, Copy, Clone)]
pub struct KeypadButtons {
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
}

/// Displays the status of keypad buttons.
///
/// Read only.
#[modular_bitfield::bitfield(bits = 16, packed = false)]
#[repr(u16)]
#[derive(Debug, Default, Copy, Clone)]
pub struct KeypadStatus {
    buttons: KeypadButtons,
    #[skip]
    unused: modular_bitfield::prelude::B6,
}

#[modular_bitfield::bitfield(bits = 16)]
#[repr(u16)]
#[derive(Debug, Copy, Clone, Default)]
pub struct KeypadInterruptControl {
    buttons: KeypadButtons,
    #[skip]
    unused: modular_bitfield::prelude::B4,
    button_irq_enable: bool,
    /// (0=Logical OR, 1=Logical AND)
    /// In logical OR mode, an interrupt is requested when at least one of the selected buttons is pressed.
    /// In logical AND mode, an interrupt is requested when ALL of the selected buttons are pressed.
    button_irq_condition: bool,
}
