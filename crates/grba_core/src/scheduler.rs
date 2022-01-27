use std::ops::{Add, AddAssign, Sub};
use std::ptr;

const MAX_EVENTS: usize = 20;

/// The `Scheduler` drives all emulation time related events.
/// Is also governs the current time.
#[derive(Debug)]
pub struct Scheduler {
    /// The current emulation time, expressed in clock cycles.
    pub current_time: EmuTime,
    /// The queue of events to be processed.
    /// The back of the queue is the next event to be processed.
    // event_queue: tinyvec::ArrayVec<[Event; 20]>,
    event_queue: [Event; MAX_EVENTS],
    event_queue_len: usize,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            current_time: 0.into(),
            event_queue: [Event::default(); MAX_EVENTS],
            event_queue_len: 0,
        }
    }

    /// The complete event queue
    pub fn event_queue(&self) -> &[Event] {
        &self.event_queue[0..self.event_queue_len]
    }

    /// Set the current time to the next closest event.
    #[inline]
    pub fn skip_to_next_event(&mut self) {
        if self.event_queue_len != 0 {
            self.current_time = self.event_queue[self.event_queue_len - 1].timestamp;
        }
    }

    /// Returns any event that should be executed at less than or equal to the current time.
    ///
    /// The event is removed from the scheduler.
    #[inline]
    pub fn pop_current(&mut self) -> Option<Event> {
        if self.event_queue_len == 0 {
            return None;
        }
        let index = self.event_queue_len - 1;
        let ev = self.event_queue[index];

        if ev.timestamp <= self.current_time {
            self.event_queue_len = index;
            Some(ev)
        } else {
            None
        }
    }

    /// Remove all events with the given tag
    pub fn remove_event(&mut self, tag: EventTag) {
        for i in 0..self.event_queue_len {
            if self.event_queue[i].tag == tag {
                self.remove_event_unsafe(i);
            }
        }
    }

    /// Removes the first event that matches the given tag.
    /// Has an early stop compared to [Self::remove_event]
    pub fn remove_first_event(&mut self, tag: EventTag) {
        for i in (0..self.event_queue_len).rev() {
            if self.event_queue[i].tag == tag {
                self.remove_event_unsafe(i);
                return;
            }
        }
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
    fn add_event(&mut self, event: Event) {
        if self.event_queue_len == 0 {
            self.event_queue[0] = event;
            self.event_queue_len += 1;
        } else {
            let current_index = self.event_queue_len - 1;
            for i in (0..self.event_queue_len).rev() {
                if event.timestamp <= self.event_queue[i].timestamp {
                    if i == current_index {
                        self.event_queue[i + 1] = event;
                        self.event_queue_len += 1;
                    } else {
                        self.insert_event(i + 1, event);
                    }

                    return;
                }
            }

            // Insert at the 'back' (due to our backwards layout this is the front).
            self.insert_event(0, event);
        }
    }

    fn insert_event(&mut self, index: usize, event: Event) {
        unsafe {
            let p = self.event_queue.as_mut_ptr().add(index);
            // Shift everything over to make space. (Duplicating the
            // `index`th element into two consecutive places.)
            ptr::copy(p, p.offset(1), self.event_queue_len - index);
            // Write it in, overwriting the first copy of the `index`th
            // element.
            ptr::write(p, event);

            self.event_queue_len += 1;
        }
    }

    fn remove_event_unsafe(&mut self, index: usize) {
        unsafe {
            // the place we are taking from.
            let ptr = self.event_queue.as_mut_ptr().add(index);

            // Shift everything down to fill in that spot.
            ptr::copy(ptr.offset(1), ptr, self.event_queue_len - index - 1);

            self.event_queue_len -= 1;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Default, serde::Serialize, serde::Deserialize)]
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
