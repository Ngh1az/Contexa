//! Frame Differencer — `docs/05_Vision_Engine.md` §5.3.

use crate::hash::{ahash, hamming};
use crate::types::Frame;

const DEFAULT_THRESHOLD: f32 = 0.05; // 5% change, per docs/05 §5.3

pub struct FrameDifferencer {
    prev_hash: Option<[u64; 4]>,
    threshold: f32,
}

impl Default for FrameDifferencer {
    fn default() -> Self {
        Self::new(DEFAULT_THRESHOLD)
    }
}

impl FrameDifferencer {
    #[must_use]
    pub fn new(threshold: f32) -> Self {
        Self {
            prev_hash: None,
            threshold,
        }
    }

    /// Returns `true` if `frame` differs from the previously seen frame by
    /// more than the threshold. The first frame seen is always "changed".
    pub fn has_significant_change(&mut self, frame: &Frame) -> bool {
        let pitch = frame.width as usize * 4;
        let hash = ahash(
            &frame.data,
            frame.width as usize,
            frame.height as usize,
            pitch,
        );
        let changed = match self.prev_hash {
            None => true,
            // hamming() is bounded to [0, 256] — exact in f32 regardless of its u32 type.
            #[allow(clippy::cast_precision_loss)]
            Some(prev) => (hamming(&prev, &hash) as f32 / 256.0) > self.threshold,
        };
        self.prev_hash = Some(hash);
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn split_frame(left_dark: bool) -> Frame {
        let (w, h) = (64u32, 64u32);
        let pitch = w as usize * 4;
        let mut data = vec![0u8; pitch * h as usize];
        let (dark, light) = (10u8, 240u8);
        for y in 0..h as usize {
            for x in 0..w as usize {
                let is_left = x < w as usize / 2;
                let v = if is_left == left_dark { dark } else { light };
                let px = &mut data[y * pitch + x * 4..y * pitch + x * 4 + 4];
                px[0] = v;
                px[1] = v;
                px[2] = v;
                px[3] = 255;
            }
        }
        Frame {
            data,
            width: w,
            height: h,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn first_frame_is_always_a_change() {
        let mut differ = FrameDifferencer::default();
        assert!(differ.has_significant_change(&split_frame(true)));
    }

    #[test]
    fn identical_consecutive_frames_are_not_a_change() {
        let mut differ = FrameDifferencer::default();
        assert!(differ.has_significant_change(&split_frame(true)));
        assert!(!differ.has_significant_change(&split_frame(true)));
    }

    #[test]
    fn flipped_frame_is_a_change() {
        let mut differ = FrameDifferencer::default();
        assert!(differ.has_significant_change(&split_frame(true)));
        assert!(differ.has_significant_change(&split_frame(false)));
    }
}
