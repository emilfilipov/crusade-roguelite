use std::thread;
use std::time::{Duration, Instant};

use bevy::prelude::*;

use crate::model::FrameRateCap;

#[derive(Resource, Debug)]
struct FrameLimiterClock {
    last_frame_mark: Instant,
}

impl Default for FrameLimiterClock {
    fn default() -> Self {
        Self {
            last_frame_mark: Instant::now(),
        }
    }
}

pub struct PerformancePlugin;

impl Plugin for PerformancePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameRateCap>()
            .init_resource::<FrameLimiterClock>()
            .add_systems(Last, limit_frame_rate);
    }
}

fn limit_frame_rate(target: Res<FrameRateCap>, mut clock: ResMut<FrameLimiterClock>) {
    if cfg!(test) {
        return;
    }

    let now = Instant::now();
    let elapsed = now.saturating_duration_since(clock.last_frame_mark);
    let target_frame = frame_duration_for_cap(*target);
    if let Some(sleep_for) = sleep_duration_for_elapsed(elapsed, target_frame) {
        thread::sleep(sleep_for);
    }
    clock.last_frame_mark = Instant::now();
}

pub fn frame_duration_for_cap(cap: FrameRateCap) -> Duration {
    Duration::from_secs_f64(1.0 / cap.as_u32() as f64)
}

pub fn sleep_duration_for_elapsed(elapsed: Duration, target_frame: Duration) -> Option<Duration> {
    if elapsed < target_frame {
        Some(target_frame - elapsed)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::model::FrameRateCap;
    use crate::performance::{frame_duration_for_cap, sleep_duration_for_elapsed};

    #[test]
    fn frame_duration_scales_with_target_fps() {
        let d60 = frame_duration_for_cap(FrameRateCap::Fps60);
        let d120 = frame_duration_for_cap(FrameRateCap::Fps120);
        assert!(d120 < d60);
    }

    #[test]
    fn sleep_duration_only_when_under_target_frame_time() {
        let target = Duration::from_millis(16);
        assert_eq!(
            sleep_duration_for_elapsed(Duration::from_millis(5), target),
            Some(Duration::from_millis(11))
        );
        assert_eq!(
            sleep_duration_for_elapsed(Duration::from_millis(16), target),
            None
        );
        assert_eq!(
            sleep_duration_for_elapsed(Duration::from_millis(20), target),
            None
        );
    }
}
