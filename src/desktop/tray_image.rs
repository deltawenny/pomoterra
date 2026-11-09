//! Circular, theme-aware countdown glyphs shared by native tray backends.
use std::sync::atomic::{AtomicU32, Ordering};

use crate::store::{Phase, TimerState};

// Publish a coherent snapshot without moving native objects off their UI thread.
// Progress uses 1/3600 of a circle, enough to reflect every second of a session.
static STATE: AtomicU32 = AtomicU32::new(3600 << 2);
const FULL: u32 = 3600;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TrayState {
    timer: u32,
    dark: bool,
}

impl TrayState {
    pub(super) fn current() -> Self {
        Self {
            timer: STATE.load(Ordering::Relaxed),
            dark: false,
        }
    }

    pub(super) fn with_dark(self, dark: bool) -> Self {
        Self { dark, ..self }
    }

    fn running(self) -> bool {
        self.timer & 1 != 0
    }

    fn progress(self) -> f32 {
        (self.timer >> 2).min(FULL) as f32 / FULL as f32
    }

    fn color(self) -> [u8; 3] {
        // Brighter tray-specific accents stay visible at small panel sizes.
        if self.timer & 2 == 0 {
            [0x29, 0xb6, 0xf6]
        } else {
            [0x2e, 0xd5, 0x91]
        }
    }
}

fn snapshot(timer: &TimerState) -> u32 {
    let phase = if timer.phase == Phase::Break { 2 } else { 0 };
    let progress = (timer.remaining as f64 / timer.duration().max(1) as f64).clamp(0.0, 1.0);
    (((progress * f64::from(FULL)).round() as u32) << 2) | phase | u32::from(timer.running)
}

pub(super) fn publish(timer: &TimerState) {
    STATE.store(snapshot(timer), Ordering::Relaxed);
}

type Point = (f32, f32);

fn distance_squared(point: Point, a: Point, b: Point) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let length = dx * dx + dy * dy;
    let t = if length == 0.0 {
        0.0
    } else {
        (((point.0 - a.0) * dx + (point.1 - a.1) * dy) / length).clamp(0.0, 1.0)
    };
    (point.0 - a.0 - t * dx).powi(2) + (point.1 - a.1 - t * dy).powi(2)
}

/// Render a circular disk with transparent corners and an actual countdown ring.
/// The ring starts at twelve o'clock and drains clockwise, like the main timer.
pub(super) fn icon_rgba(size: usize, state: TrayState) -> Result<Vec<u8>, String> {
    if size == 0 || size > 256 {
        return Err("Tray icon size must be between 1 and 256".into());
    }
    let glyph: Vec<(Point, Point)> = if state.running() {
        vec![((38.0, 33.0), (38.0, 63.0)), ((58.0, 33.0), (58.0, 63.0))]
    } else {
        vec![
            ((38.0, 31.0), (64.0, 48.0)),
            ((64.0, 48.0), (38.0, 65.0)),
            ((38.0, 65.0), (38.0, 31.0)),
        ]
    };
    let mut pixels = vec![0; size * size * 4];
    let color = state.color();
    let background = if state.dark {
        [0, 0, 0]
    } else {
        [255, 255, 255]
    };
    let track = if state.dark {
        [72, 72, 72]
    } else {
        [237, 243, 247]
    };
    let half_stroke = (96.0 / size as f32 * 0.75).max(1.8);
    // Give the progress ring more weight than the central playback glyph.
    let ring_width = (96.0 / size as f32 * 2.0).max(6.0);
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0_u32; 3];
            let mut covered = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let point = (
                        (x as f32 + (sx as f32 + 0.5) / 4.0) * 96.0 / size as f32,
                        (y as f32 + (sy as f32 + 0.5) / 4.0) * 96.0 / size as f32,
                    );
                    let dx = point.0 - 48.0;
                    let dy = point.1 - 48.0;
                    let radius = dx.hypot(dy);
                    if radius > 46.0 {
                        continue;
                    }
                    let angle = (dy.atan2(dx) + std::f32::consts::FRAC_PI_2)
                        .rem_euclid(std::f32::consts::TAU);
                    let ring = radius >= 46.0 - ring_width;
                    let foreground = (ring && angle <= std::f32::consts::TAU * state.progress())
                        || glyph.iter().any(|&(a, b)| {
                            distance_squared(point, a, b) <= half_stroke * half_stroke
                        });
                    let sample = if foreground {
                        color
                    } else if ring {
                        track
                    } else {
                        background
                    };
                    for channel in 0..3 {
                        sum[channel] += u32::from(sample[channel]);
                    }
                    covered += 1;
                }
            }
            let pixel = &mut pixels[(y * size + x) * 4..][..4];
            if covered > 0 {
                for channel in 0..3 {
                    pixel[channel] = (sum[channel] / covered) as u8;
                }
                pixel[3] = (covered * 255 / 16) as u8;
            }
        }
    }
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(flags: u32, progress: u32, dark: bool) -> TrayState {
        TrayState {
            timer: (progress << 2) | flags,
            dark,
        }
    }

    #[test]
    fn icons_are_round_at_each_size_and_theme() {
        for flags in 0..4 {
            for dark in [false, true] {
                for size in [16, 24, 32, 48, 64] {
                    let pixels = icon_rgba(size, state(flags, FULL, dark)).unwrap();
                    assert_eq!(pixels.len(), size * size * 4);
                    assert_eq!(pixels[3], 0);
                    assert!(pixels.chunks_exact(4).any(|p| p[3] == 255));
                    let interior = &pixels[((size / 2) * size + size / 4) * 4..][..4];
                    assert_eq!(
                        interior,
                        if dark {
                            &[0, 0, 0, 255]
                        } else {
                            &[255, 255, 255, 255]
                        }
                    );
                }
            }
        }
        assert!(icon_rgba(0, state(0, FULL, false)).is_err());
        assert!(icon_rgba(257, state(0, FULL, false)).is_err());
    }

    #[test]
    fn progress_playback_phase_and_theme_each_change_the_icon() {
        let full = icon_rgba(32, state(0, FULL, false)).unwrap();
        for other in [
            state(1, FULL, false),
            state(2, FULL, false),
            state(0, FULL / 2, false),
            state(0, 0, false),
            state(0, FULL, true),
        ] {
            assert_ne!(full, icon_rgba(32, other).unwrap());
        }
    }

    #[test]
    fn snapshot_tracks_remaining_time_even_when_paused() {
        let mut timer = TimerState::default();
        assert_eq!(snapshot(&timer) >> 2, FULL);
        timer.remaining /= 2;
        assert_eq!(snapshot(&timer) >> 2, FULL / 2);
        timer.running = true;
        assert_eq!(snapshot(&timer) & 1, 1);
        timer.remaining = 0;
        assert_eq!(snapshot(&timer) >> 2, 0);
    }
}
