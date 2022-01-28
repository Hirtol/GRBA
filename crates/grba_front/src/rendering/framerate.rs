use std::time::{Duration, Instant};

/// Simple moving average implementation
pub struct FrameRate {
    /// The current sum of frame rates
    frame_sum: Duration,
    /// The saved frame rates used for the moving average
    frame_lengths: [Duration; 120],
    /// The current index for the [f
    index: usize,
    /// The last time the frame rate was updated.
    last_time: Instant,
}

impl FrameRate {
    /// Creates a new frame rate.
    pub fn new() -> Self {
        Self {
            frame_sum: Duration::from_secs(0),
            frame_lengths: [Duration::from_secs(0); 120],
            index: 0,
            last_time: Instant::now(),
        }
    }

    /// Updates the frame rate.
    pub fn frame_finished(&mut self) {
        let now = Instant::now();
        let delta = now - self.last_time;
        // let new_fps = 1.0 / delta.as_secs_f32();
        self.frame_sum -= self.frame_lengths[self.index];
        self.frame_sum += delta;
        self.frame_lengths[self.index] = delta;
        self.index += 1;

        if self.index >= self.frame_lengths.len() {
            self.index = 0;
        }

        self.last_time = now;
    }

    pub fn fps(&self) -> f32 {
        1.0 / (self.frame_sum.as_secs_f32() / self.frame_lengths.len() as f32)
    }
}
