//! Frame delays in GIF centiseconds.

/// How per-frame delays are chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DelayMode {
    /// Delays alternate (e.g. 8, 9, 8 cs at 12 fps) so the time after `k`
    /// frames is `100 k / fps` rounded to the nearest centisecond. The loop
    /// plays at the requested rate. Deviation from v2, chosen for the rewrite.
    #[default]
    Accumulated,
    /// v2's constant `GifSettings::frame_delay_cs` for every frame
    /// (`clamp(round(100 / fps), 2, 100)`), which drifts from the requested
    /// rate when 100 / fps is not a whole number.
    Constant,
}

/// Shortest delay browsers honour; shorter ones are often shown slower.
pub const MIN_DELAY_CS: u16 = 2;

/// Delay of each of `frames` frames at `fps` (fps below 1 counts as 1).
///
/// Accumulated: delay `i` is `end(i + 1) - end(i)` with
/// `end(k) = round(100 k / fps)` (halves round up), at least
/// [`MIN_DELAY_CS`]. Integer arithmetic only, so it is exact and
/// deterministic. The minimum only bites above 50 fps, outside the schema range.
pub fn frame_delays(fps: i32, frames: usize, mode: DelayMode) -> Vec<u16> {
    let fps = u64::from(fps.max(1).unsigned_abs());
    match mode {
        DelayMode::Constant => {
            // Same expression as `GifSettings::frame_delay_cs`.
            let cs = ((100.0 / fps as f64).round() as u16).clamp(MIN_DELAY_CS, 100);
            vec![cs; frames]
        }
        DelayMode::Accumulated => {
            let end = |k: u64| (200 * k + fps) / (2 * fps);
            (0..frames as u64)
                .map(|i| ((end(i + 1) - end(i)) as u16).max(MIN_DELAY_CS))
                .collect()
        }
    }
}
