use binary_heap_plus::{BinaryHeap, MinComparator};
use std::cmp::Ordering;
use std::ops::{Add, AddAssign, Sub};

const MAX_EVENTS: usize = 20;

/// The `Scheduler` drives all emulation time related events.
/// Is also governs the current time.
#[derive(Debug)]
pub struct Scheduler {
    /// The current emulation time, expressed in clock cycles.
    pub current_time: EmuTime,
    /// The queue of events to be processed.
    /// The back of the queue is the next event to be processed.
    event_queue: BinaryHeap<Event, MinComparator>,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            current_time: 0u32.into(),
            event_queue: BinaryHeap::with_capacity_min(MAX_EVENTS),
        }
    }

    /// Set the current time to the next closest event.
    #[inline]
    pub fn skip_to_next_event(&mut self) {
        if let Some(ev) = self.event_queue.peek() {
            // We need the modulo 4, since events could be scheduled at times when they're
            // not aligned on proper t-cycle boundaries.
            self.current_time = ev.timestamp;
        }
    }

    /// Returns any event that should be executed at less than or equal to the current time.
    ///
    /// The event is removed from the scheduler.
    #[inline]
    pub fn pop_current(&mut self) -> Option<Event> {
        if self
            .event_queue
            .peek()
            .map_or(false, |ev| ev.timestamp <= self.current_time)
        {
            self.event_queue.pop()
        } else {
            None
        }
    }

    /// Remove all events with the given tag
    pub fn remove_event(&mut self, tag: EventTag) {
        let mut current_vec = std::mem::replace(&mut self.event_queue, BinaryHeap::new_min()).into_vec();
        current_vec.retain(|e| e.tag != tag);
        self.event_queue = BinaryHeap::from_vec(current_vec);
    }

    /// Clears all scheduled events.
    pub fn clear_events(&mut self) {
        self.event_queue.clear();
    }

    #[inline(always)]
    pub fn add_time(&mut self, time: i32) {
        self.current_time.0 += time as u64;
    }

    /// Schedule the provided event at the absolute time `when`.
    pub fn schedule_event(&mut self, tag: EventTag, when: EmuTime) {
        self.add_event(Event { tag, timestamp: when });
    }

    /// Schedule the provided event, at the absolute time `when + current_time`.
    pub fn schedule_relative(&mut self, tag: EventTag, when: EmuTime) {
        self.add_event(Event {
            tag,
            timestamp: self.current_time + when,
        });
    }

    /// Adds the provided event.
    ///
    /// The events are laid out from back to front, based on their timestamp. (Earlier is at back of array)
    #[inline(always)]
    #[profiling::function]
    fn add_event(&mut self, event: Event) {
        self.event_queue.push(event);
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Scheduler::new()
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum EventTag {
    /// Shouldn't ever be reached, but will ensure the emulator will exit correctly.
    Exit,

    VBlank,
    HBlank,
    HBlankEnd,
    /// Check for interrupts, will be set whenever an interrupt write happens to ensure the CPU can handle it.
    PollInterrupt,
    /// Start the `Halted` loop, which is only stopped when an enabled interrupt is fired.
    Halt,
    Timer0Irq,
    Timer1Irq,
    Timer2Irq,
    Timer3Irq,
}

#[derive(Debug, Copy, Clone, Eq)]
pub struct Event {
    pub tag: EventTag,
    pub timestamp: EmuTime,
}

impl Default for Event {
    fn default() -> Self {
        Event {
            tag: EventTag::Exit,
            timestamp: EmuTime(u64::MAX),
        }
    }
}

impl core::cmp::PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl core::cmp::PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.timestamp.partial_cmp(&other.timestamp)
    }
}

impl core::cmp::Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

/// Represents a certain point in time, expressed in emulator clock cycles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Default, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct EmuTime(pub u64);

impl From<u64> for EmuTime {
    fn from(time: u64) -> Self {
        EmuTime(time)
    }
}

impl From<u32> for EmuTime {
    fn from(time: u32) -> Self {
        EmuTime(time as u64)
    }
}

impl From<u16> for EmuTime {
    fn from(time: u16) -> Self {
        EmuTime(time as u64)
    }
}

impl Add<EmuTime> for EmuTime {
    type Output = EmuTime;

    fn add(self, rhs: EmuTime) -> Self::Output {
        EmuTime(self.0 + rhs.0)
    }
}

impl AddAssign<EmuTime> for EmuTime {
    fn add_assign(&mut self, rhs: EmuTime) {
        self.0 += rhs.0;
    }
}

impl Sub<EmuTime> for EmuTime {
    type Output = EmuTime;

    fn sub(self, rhs: EmuTime) -> Self::Output {
        EmuTime(self.0 - rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::scheduler::EventTag::{HBlank, VBlank};
    use crate::scheduler::{EmuTime, Event, EventTag, Scheduler};

    #[test]
    fn test_scheduler_basic() {
        let mut scheduler = Scheduler::new();
        // Start with an empty scheduler.
        assert!(scheduler.pop_current().is_none());
        assert_eq!(scheduler.current_time, EmuTime(0));
        // If no events are scheduled this should be a no-op
        scheduler.skip_to_next_event();
        assert_eq!(scheduler.current_time, EmuTime(0));
    }

    #[test]
    fn test_scheduler_schedule() {
        let mut scheduler = Scheduler::new();
        let scheduled_event = Event {
            tag: EventTag::VBlank,
            timestamp: EmuTime(10),
        };

        scheduler.schedule_event(scheduled_event.tag, scheduled_event.timestamp);
        // Schedule in future shouldn't return quite yet.
        assert!(scheduler.pop_current().is_none());
        // Should now skip to VBlank event
        scheduler.skip_to_next_event();
        assert_eq!(scheduler.pop_current(), Some(scheduled_event));
        // Should now be empty
        assert!(scheduler.pop_current().is_none());
    }

    #[test]
    fn test_scheduler_several() {
        let mut scheduler = Scheduler::new();

        scheduler.schedule_event(EventTag::HBlank, EmuTime(10));
        scheduler.schedule_event(EventTag::VBlank, EmuTime(0));
        scheduler.schedule_event(EventTag::VBlank, EmuTime(15));

        // First VBlank should be popped
        assert_eq!(
            scheduler.pop_current(),
            Some(Event {
                tag: VBlank,
                timestamp: EmuTime(0),
            })
        );

        // Should now skip to HBlank event
        scheduler.skip_to_next_event();
        assert_eq!(
            scheduler.pop_current(),
            Some(Event {
                tag: HBlank,
                timestamp: EmuTime(10),
            })
        );

        // Last VBlank
        scheduler.skip_to_next_event();
        assert_eq!(
            scheduler.pop_current(),
            Some(Event {
                tag: VBlank,
                timestamp: EmuTime(15),
            })
        );

        scheduler.skip_to_next_event();
        assert!(scheduler.pop_current().is_none());
    }
}
