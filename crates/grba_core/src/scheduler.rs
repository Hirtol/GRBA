use std::ops::{Add, AddAssign, Sub};

const MAX_EVENTS: usize = 20;

/// The `Scheduler` drives all emulation time related events.
/// Is also governs the current time.
#[derive(Debug)]
pub struct Scheduler {
    /// The current emulation time, expressed in clock cycles.
    pub current_time: EmuTime,
    /// The queue of events to be processed.
    /// The head of the queue is the next event to be processed.
    event_queue: tinyvec::ArrayVec<[Event; 20]>,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            current_time: 0.into(),
            event_queue: tinyvec::array_vec!([Event; MAX_EVENTS]),
        }
    }

    /// The complete event queue
    pub fn event_queue(&self) -> &tinyvec::ArrayVec<[Event; 20]> {
        &self.event_queue
    }

    /// Set the current time to the next closest event.
    #[inline]
    pub fn skip_to_next_event(&mut self) {
        if let Some(ev) = self.event_queue.last() {
            self.current_time = ev.timestamp;
        }
    }

    /// Returns any event that should be executed at less than or equal to the current time.
    ///
    /// The event is removed from the scheduler.
    #[inline]
    pub fn pop_current(&mut self) -> Option<Event> {
        if matches!(self.event_queue.last(), Some(ev) if ev.timestamp <= self.current_time) {
            self.event_queue.pop()
        } else {
            None
        }
    }

    /// Remove all events with the given tag
    pub fn remove_event(&mut self, tag: EventTag) {
        self.event_queue.retain(|ev| ev.tag != tag);
    }

    /// Removes the first event that matches the given tag.
    /// Has an early stop compared to [Self::remove_event]
    pub fn remove_first_event(&mut self, tag: EventTag) {
        for i in 0..self.event_queue.len() {
            if self.event_queue[i].tag == tag {
                self.event_queue.remove(i);
                return;
            }
        }
    }

    /// Schedule the provided event at the absolute time `when`.
    pub fn schedule_event(&mut self, tag: EventTag, when: EmuTime) {
        self.add_event(Event {
            tag,
            timestamp: when,
        });
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
    fn add_event(&mut self, event: Event) {
        if self.event_queue.is_empty() {
            self.event_queue.push(event);
        } else {
            for i in (0..self.event_queue.len()).rev() {
                if event.timestamp <= self.event_queue[i].timestamp {
                    if i == self.event_queue.len() - 1 {
                        self.event_queue.push(event);
                    } else {
                        self.event_queue.insert(i + 1, event);
                    }
                    return;
                }
            }

            // Insert at the 'back' (due to our backwards layout this is the front).
            self.event_queue.insert(0, event);
        }
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
}

#[derive(Debug, Copy, Clone, PartialEq)]
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

/// Represents a certain point in time, expressed in emulator clock cycles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Default)]
#[repr(transparent)]
pub struct EmuTime(pub u64);

impl From<u64> for EmuTime {
    fn from(time: u64) -> Self {
        EmuTime(time)
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
        // First event should be popped
        assert_eq!(
            scheduler.pop_current(),
            Some(Event {
                tag: VBlank,
                timestamp: EmuTime(0)
            })
        );
        // Should now skip to HBlank event
        scheduler.skip_to_next_event();
        assert_eq!(
            scheduler.pop_current(),
            Some(Event {
                tag: HBlank,
                timestamp: EmuTime(10)
            })
        );
        // Last VBlank
        scheduler.skip_to_next_event();
        assert_eq!(
            scheduler.pop_current(),
            Some(Event {
                tag: VBlank,
                timestamp: EmuTime(15)
            })
        );

        scheduler.skip_to_next_event();
        assert!(scheduler.pop_current().is_none());
    }
}
