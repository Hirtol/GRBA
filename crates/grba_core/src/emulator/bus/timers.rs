use crate::emulator::bus::interrupts::{InterruptManager, Interrupts};
use crate::emulator::{AlignedAddress, MemoryAddress};
use crate::scheduler::{EmuTime, EventTag, Scheduler};
use crate::utils::BitOps;
use modular_bitfield::prelude::*;

pub const TIMER_IO_START: MemoryAddress = 0x0400_0100;
pub const TIMER_IO_END: MemoryAddress = 0x0400_010F;

pub struct Timers {
    timers: [Timer; 4],
}

impl Timers {
    const TIMER_EVENTS: [EventTag; 4] = [
        EventTag::Timer0Irq,
        EventTag::Timer1Irq,
        EventTag::Timer2Irq,
        EventTag::Timer3Irq,
    ];

    const TIMER_INTERRUPTS: [Interrupts; 4] = [
        Interrupts::Timer0,
        Interrupts::Timer1,
        Interrupts::Timer2,
        Interrupts::Timer3,
    ];

    pub fn new() -> Self {
        Self {
            timers: [Timer::default(), Timer::default(), Timer::default(), Timer::default()],
        }
    }

    pub fn read_registers(&mut self, addr: AlignedAddress, scheduler: &mut Scheduler) -> u8 {
        let timer_idx = Self::addr_to_timer_idx(addr);
        let timer = &mut self.timers[timer_idx];
        let timer_addr = addr as usize % 4;

        match timer_addr {
            0..=1 => {
                // Timers are not ticked when in cascade mode directly, and if the timer is not enabled then we just return the latest data
                if timer.control.cascade_mode() || !timer.control.enabled() {
                    timer.value.to_le_bytes()[timer_addr % 2]
                } else {
                    let value = timer.calculate_current_value(scheduler.current_time);

                    value.to_le_bytes()[timer_addr]
                }
            }
            2..=3 => timer.control.to_le_bytes()[timer_addr - 2],
            _ => unreachable!(),
        }
    }

    pub fn write_registers(&mut self, addr: AlignedAddress, value: u8, scheduler: &mut Scheduler) {
        let timer_idx = Self::addr_to_timer_idx(addr);
        let timer = &mut self.timers[timer_idx];
        let timer_addr = addr as usize % 4;

        match timer_addr {
            0..=1 => {
                timer.load_value = timer.load_value.change_byte_le(timer_addr, value);
            }
            2..=3 => {
                let old_cnt = timer.control;
                // Update the current value since we're going to reschedule due to potential clock tick rate changes.
                if old_cnt.enabled() && !old_cnt.cascade_mode() {
                    timer.value = timer.calculate_current_value(scheduler.current_time);
                    scheduler.remove_event(Self::TIMER_EVENTS[timer_idx]);
                }

                timer.control.update_byte_le(timer_addr - 2, value);

                if timer.control.enabled() {
                    if !old_cnt.enabled() {
                        timer.value = timer.load_value;
                    }

                    // Schedule an overflow if not cascading
                    if !timer.control.cascade_mode() {
                        timer.starting_timestamp = scheduler.current_time;
                        let overflow_time = timer.calculate_overflow_time();
                        scheduler.schedule_relative(Self::TIMER_EVENTS[timer_idx], overflow_time);
                    }
                } else if old_cnt.enabled() {
                    // Freeze the current time value as the new timer is disabled.
                    if !timer.control.cascade_mode() {
                        timer.value = timer.calculate_current_value(scheduler.current_time);
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn timer_overflowed(
        &mut self,
        timer_idx: usize,
        scheduler: &mut Scheduler,
        interrupt_time: EmuTime,
        irq: &mut InterruptManager,
    ) {
        // Reload with our load value
        self.timers[timer_idx].value = self.timers[timer_idx].load_value;

        // Check if next timer is cascading/needs to overflow as well
        if timer_idx < 3 {
            let next_timer_idx = timer_idx + 1;
            // TODO: Verify if disabled timers are skipped for cascade
            if self.timers[next_timer_idx].control.cascade_mode() && self.timers[next_timer_idx].control.enabled() {
                let (value, overflowed) = self.timers[next_timer_idx].value.overflowing_add(1);

                if overflowed {
                    self.timer_overflowed(next_timer_idx, scheduler, interrupt_time, irq);
                } else {
                    self.timers[next_timer_idx].value = value;
                }
            }
        }

        if self.timers[timer_idx].control.irq_on_overflow() {
            irq.request_interrupt(Self::TIMER_INTERRUPTS[timer_idx], scheduler);
        }

        let timer = &mut self.timers[timer_idx];

        // Schedule the next interrupt
        if !timer.control.cascade_mode() {
            let overflow_time = timer.calculate_overflow_time();
            // We don't schedule_relative here to ensure that the timers don't drift due to delayed schedule event handling.
            scheduler.schedule_event(Self::TIMER_EVENTS[timer_idx], interrupt_time + overflow_time);
            timer.starting_timestamp = interrupt_time;
        }
    }

    #[inline(always)]
    const fn addr_to_timer_idx(addr: AlignedAddress) -> usize {
        (addr - TIMER_IO_START) as usize / 4
    }
}

struct Timer {
    control: TimerControl,
    value: u16,
    load_value: u16,
    starting_timestamp: EmuTime,
}

impl Timer {
    #[inline]
    pub fn calculate_current_value(&self, current_timestamp: EmuTime) -> u16 {
        let ticks_passed =
            ((current_timestamp - self.starting_timestamp).0 / self.control.timer_frequency().to_ticks()) as u16;

        self.value.wrapping_add(ticks_passed)
    }

    #[inline]
    pub fn calculate_overflow_time(&self) -> EmuTime {
        EmuTime((u16::MAX - self.value) as u64 * self.control.timer_frequency().to_ticks())
    }
}

impl Default for Timer {
    fn default() -> Self {
        Timer {
            control: Default::default(),
            value: 0,
            load_value: 0,
            starting_timestamp: Default::default(),
        }
    }
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[derive(Default, Copy, Clone)]
struct TimerControl {
    timer_frequency: TimerFrequency,
    cascade_mode: bool,
    #[skip]
    _unused: B3,
    irq_on_overflow: bool,
    enabled: bool,
    #[skip]
    _unused2: u8,
}

#[derive(Debug, BitfieldSpecifier)]
#[bits = 2]
pub enum TimerFrequency {
    C1 = 0b00,
    C64 = 0b01,
    C256 = 0b10,
    C1024 = 0b11,
}

impl TimerFrequency {
    pub fn to_ticks(&self) -> u64 {
        match self {
            TimerFrequency::C1 => 1,
            TimerFrequency::C64 => 64,
            TimerFrequency::C256 => 256,
            TimerFrequency::C1024 => 1024,
        }
    }
}
