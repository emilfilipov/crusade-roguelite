use std::thread;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

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

#[derive(Resource, Debug)]
struct FpsCounterRuntime {
    last_frame_mark: Instant,
    sample_elapsed: f32,
    sample_frames: u32,
    displayed_fps: f32,
}

impl Default for FpsCounterRuntime {
    fn default() -> Self {
        Self {
            last_frame_mark: Instant::now(),
            sample_elapsed: 0.0,
            sample_frames: 0,
            displayed_fps: 0.0,
        }
    }
}

const WINDOW_TITLE_BASE: &str = "Crusade Roguelite";
const FPS_SAMPLE_WINDOW_SECS: f32 = 0.33;

pub struct PerformancePlugin;

impl Plugin for PerformancePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameRateCap>()
            .init_resource::<FrameLimiterClock>()
            .init_resource::<FpsCounterRuntime>()
            .add_systems(Update, update_window_title_with_fps)
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

fn update_window_title_with_fps(
    mut runtime: ResMut<FpsCounterRuntime>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if cfg!(test) {
        return;
    }

    let now = Instant::now();
    let dt = now
        .saturating_duration_since(runtime.last_frame_mark)
        .as_secs_f32();
    runtime.last_frame_mark = now;
    if dt <= 0.0 {
        return;
    }

    runtime.sample_elapsed += dt;
    runtime.sample_frames = runtime.sample_frames.saturating_add(1);
    if runtime.sample_elapsed >= FPS_SAMPLE_WINDOW_SECS {
        runtime.displayed_fps = runtime.sample_frames as f32 / runtime.sample_elapsed;
        runtime.sample_elapsed = 0.0;
        runtime.sample_frames = 0;
    }

    if let Ok(mut window) = windows.get_single_mut() {
        window.title = format_window_title(runtime.displayed_fps);
    }
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

pub fn format_window_title(fps: f32) -> String {
    if fps <= 0.0 {
        return WINDOW_TITLE_BASE.to_string();
    }
    format!("{WINDOW_TITLE_BASE} | FPS: {:.0}", fps)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::model::FrameRateCap;
    use crate::performance::{
        format_window_title, frame_duration_for_cap, sleep_duration_for_elapsed,
    };

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

    #[test]
    fn window_title_includes_fps_when_available() {
        assert_eq!(format_window_title(0.0), "Crusade Roguelite");
        assert_eq!(format_window_title(61.2), "Crusade Roguelite | FPS: 61");
    }
}
