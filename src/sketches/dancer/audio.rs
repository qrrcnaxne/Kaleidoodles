use std::{
    collections::VecDeque,
    thread::{self, JoinHandle},
    time::Duration,
};

use bevy::prelude::*;
use cpal::{
    Sample,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossbeam_channel::{Receiver, Sender, TryRecvError, bounded, unbounded};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::recording::Recording;

const RMS_WINDOWS_PER_SECOND: usize = 50;
const RMS_HISTORY_SECONDS: usize = 8;
const MIN_ANALYSIS_SECONDS: usize = 1;
const MIN_ONSETS_FOR_ESTIMATE: usize = 4;
const LOCK_CONFIRMATION_WINDOWS: usize = 2;
const MAX_UNSTABLE_CANDIDATE_WINDOWS: usize = 2;
const ESTIMATE_INTERVAL_SECONDS: usize = 1;
const MIN_BPM: f32 = 60.0;
const MAX_BPM: f32 = 180.0;
const MIN_MANUAL_BPM: f32 = 30.0;
const MAX_MANUAL_BPM: f32 = 300.0;
const MIN_CONFIDENCE: f32 = 0.35;
const BPM_CANDIDATE_TOLERANCE: f32 = 0.06;
const MIN_ONSET_RISE: f32 = 0.001;
const RELATIVE_ONSET_RISE: f32 = 0.08;
const MIN_ACTIVE_RMS: f32 = 0.003;
const SILENCE_TIMEOUT_SECONDS: usize = 1;
const NO_BEAT_TIMEOUT_SECONDS: usize = 2;
const MAX_LOW_CONFIDENCE_WINDOWS: usize = 2;
const BPM_SMOOTHING: f32 = 0.2;
const SAMPLE_BUFFER_MILLISECONDS: usize = 250;
const BPM_HISTORY_LENGTH: usize = 5;
const VOLUME_HISTORY_WINDOWS: usize = 2 * RMS_WINDOWS_PER_SECOND;
const VOLUME_CALIBRATION_WINDOWS: usize = VOLUME_HISTORY_WINDOWS;
const VOLUME_GATE_OPEN_RATIO: f32 = 1.7;
const VOLUME_GATE_CLOSE_RATIO: f32 = 1.3;
const VOLUME_GATE_MIN_OPEN_RMS: f32 = 0.008;
const VOLUME_GATE_MIN_CLOSE_RMS: f32 = 0.006;
const VOLUME_GATE_CLOSE_DELAY_FRAMES: usize = 3 * RMS_WINDOWS_PER_SECOND / 2;
const NOISE_FLOOR_RISE_ALPHA: f32 = 0.02;
const NOISE_FLOOR_FALL_ALPHA: f32 = 0.2;
const AUDIO_LEVEL_SMOOTHING: f32 = 0.2;

pub(super) struct MicrophonePlugin;

impl Plugin for MicrophonePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TempoSource::from_args())
            .init_resource::<MicrophoneRuntime>()
            .add_systems(Startup, start_microphone)
            .add_systems(PreUpdate, receive_microphone_updates);
    }
}

#[derive(Resource, Default)]
pub(super) struct TempoSource {
    pub(super) bpm: Option<f32>,
    using_recent_average: bool,
    manual: bool,
}

impl TempoSource {
    fn from_args() -> Self {
        let mut args = std::env::args().skip(1);
        let mut bpm = None;
        while let Some(arg) = args.next() {
            if arg == "--bpm" {
                bpm = args.next().as_deref().and_then(parse_manual_bpm);
                if bpm.is_none() {
                    bevy::log::warn!("Expected --bpm <value> between 30 and 300.");
                }
            }
        }

        if let Some(bpm) = bpm {
            bevy::log::info!(
                "Using command-line tempo {bpm:.1} BPM; microphone tracking is disabled."
            );
        }
        Self {
            bpm,
            using_recent_average: false,
            manual: bpm.is_some(),
        }
    }
}

fn parse_manual_bpm(value: &str) -> Option<f32> {
    let bpm = value.parse::<f32>().ok()?;
    (bpm.is_finite() && (MIN_MANUAL_BPM..=MAX_MANUAL_BPM).contains(&bpm)).then_some(bpm)
}

#[derive(Resource)]
struct MicrophoneRuntime {
    updates: Receiver<MicrophoneUpdate>,
    update_sender: Option<Sender<MicrophoneUpdate>>,
    stop_sender: Option<Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl Default for MicrophoneRuntime {
    fn default() -> Self {
        let (update_sender, updates) = unbounded();
        Self {
            updates,
            update_sender: Some(update_sender),
            stop_sender: None,
            worker: None,
        }
    }
}

impl Drop for MicrophoneRuntime {
    fn drop(&mut self) {
        if let Some(stop_sender) = self.stop_sender.take() {
            let _ = stop_sender.try_send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[derive(Debug)]
enum MicrophoneUpdate {
    Ready {
        device_name: String,
        sample_format: cpal::SampleFormat,
        sample_rate: u32,
        channels: u16,
    },
    Tempo {
        bpm: Option<f32>,
        using_recent_average: bool,
    },
    Unavailable(String),
}

struct MicrophoneInput {
    _stream: cpal::Stream,
    samples: Consumer<f32>,
    stream_errors: Receiver<cpal::StreamError>,
    device_name: String,
    sample_format: cpal::SampleFormat,
    sample_rate: u32,
    channels: u16,
}

fn start_microphone(
    recording: Option<Res<Recording>>,
    tempo: Res<TempoSource>,
    mut runtime: ResMut<MicrophoneRuntime>,
) {
    if recording.is_some() || tempo.manual {
        return;
    }

    let Some(update_sender) = runtime.update_sender.take() else {
        return;
    };
    let (stop_sender, stop_receiver) = bounded(1);
    let worker = match thread::Builder::new()
        .name("dancer-microphone".to_owned())
        .spawn(move || microphone_worker(update_sender, stop_receiver))
    {
        Ok(worker) => worker,
        Err(error) => {
            bevy::log::warn!("Could not start microphone worker: {error}");
            return;
        }
    };

    runtime.stop_sender = Some(stop_sender);
    runtime.worker = Some(worker);
}

fn receive_microphone_updates(runtime: Res<MicrophoneRuntime>, mut tempo: ResMut<TempoSource>) {
    while let Ok(update) = runtime.updates.try_recv() {
        match update {
            MicrophoneUpdate::Ready {
                device_name,
                sample_format,
                sample_rate,
                channels,
            } => bevy::log::info!(
                "Microphone input active: {device_name} ({sample_rate} Hz, {channels} channels, {sample_format:?}); waiting for a stable beat."
            ),
            MicrophoneUpdate::Tempo {
                bpm,
                using_recent_average,
            } => {
                let previous = tempo.bpm;
                let was_using_recent_average = tempo.using_recent_average;
                tempo.bpm = bpm;
                tempo.using_recent_average = bpm.is_some() && using_recent_average;
                match (bpm, using_recent_average) {
                    (Some(bpm), true)
                        if !was_using_recent_average
                            || previous.is_none_or(|previous| (previous - bpm).abs() >= 1.0) =>
                    {
                        bevy::log::info!(
                            "Music is active without a stable beat; using recent tempo average {bpm:.1} BPM."
                        );
                    }
                    (Some(bpm), false)
                        if was_using_recent_average
                            || previous.is_none_or(|previous| (previous - bpm).abs() >= 1.0) =>
                    {
                        bevy::log::info!("Dance beat locked at {bpm:.1} BPM.");
                    }
                    (None, _) if previous.is_some() => {
                        bevy::log::info!(
                            "Music fell below the adaptive volume gate; returning the dancer to neutral."
                        );
                    }
                    _ => {}
                }
            }
            MicrophoneUpdate::Unavailable(reason) => {
                tempo.bpm = None;
                tempo.using_recent_average = false;
                bevy::log::warn!(
                    "Microphone unavailable; the dancer and disco lights will stay stopped: {reason}"
                );
            }
        }
    }
}

fn microphone_worker(update_sender: Sender<MicrophoneUpdate>, stop_receiver: Receiver<()>) {
    let mut input = match open_default_microphone() {
        Ok(input) => input,
        Err(reason) => {
            let _ = update_sender.send(MicrophoneUpdate::Unavailable(reason));
            return;
        }
    };

    let _ = update_sender.send(MicrophoneUpdate::Ready {
        device_name: input.device_name.clone(),
        sample_format: input.sample_format,
        sample_rate: input.sample_rate,
        channels: input.channels,
    });

    let samples_per_window = (input.sample_rate as usize / RMS_WINDOWS_PER_SECOND)
        .max(1)
        .saturating_mul(usize::from(input.channels));
    let mut tracker = BpmTracker::default();
    let mut volume_gate = AudioPresenceGate::default();
    let mut recent_bpms = VecDeque::with_capacity(BPM_HISTORY_LENGTH);
    let mut square_sum = 0.0_f64;
    let mut sample_count = 0_usize;

    loop {
        if sample_count == 0 {
            if stop_requested(&stop_receiver) {
                return;
            }
            if let Ok(error) = input.stream_errors.try_recv() {
                let _ = update_sender.send(MicrophoneUpdate::Unavailable(format!(
                    "Input stream error: {error}"
                )));
                return;
            }
        }

        let sample = match input.samples.pop() {
            Ok(sample) => sample,
            Err(_) => {
                thread::sleep(Duration::from_millis(1));
                continue;
            }
        };
        let sample = f64::from(sample);
        square_sum += sample * sample;
        sample_count += 1;

        if sample_count >= samples_per_window {
            let rms = (square_sum / sample_count as f64).sqrt() as f32;
            let was_present = volume_gate.is_active();
            volume_gate.push_rms(rms);
            let tracker_update = tracker.push_rms(rms);

            if let Some(TrackerUpdate::Locked(bpm)) = tracker_update {
                volume_gate.confirm_audio();
                push_recent_bpm(&mut recent_bpms, bpm);
            }

            let presence_changed = was_present != volume_gate.is_active();
            if was_present && !volume_gate.is_active() {
                tracker.reset(rms);
                recent_bpms.clear();
            }

            if presence_changed || tracker_update.is_some() {
                let (bpm, using_recent_average) =
                    select_tempo(&tracker, &volume_gate, &recent_bpms);
                let _ = update_sender.send(MicrophoneUpdate::Tempo {
                    bpm,
                    using_recent_average,
                });
            }

            square_sum = 0.0;
            sample_count = 0;
        }
    }
}

fn open_default_microphone() -> Result<MicrophoneInput, String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "No default microphone input device is available.".to_owned())?;
    let device_name = device
        .description()
        .map(|description| description.name().to_owned())
        .unwrap_or_else(|_| "unnamed input".to_owned());
    let supported_config = device
        .default_input_config()
        .map_err(|error| error.to_string())?;
    let sample_rate = supported_config.sample_rate();
    let channels = supported_config.channels();
    let sample_format = supported_config.sample_format();
    let stream_config: cpal::StreamConfig = supported_config.into();
    let capacity = (sample_rate as usize)
        .saturating_mul(usize::from(channels))
        .saturating_mul(SAMPLE_BUFFER_MILLISECONDS)
        .div_ceil(1_000)
        .max(1);
    let (sample_producer, samples) = RingBuffer::<f32>::new(capacity);
    let (error_sender, stream_errors) = bounded(1);
    let stream = build_stream(
        &device,
        &stream_config,
        sample_format,
        sample_producer,
        error_sender,
    )?;
    stream.play().map_err(|error| error.to_string())?;

    Ok(MicrophoneInput {
        _stream: stream,
        samples,
        stream_errors,
        device_name,
        sample_format,
        sample_rate,
        channels,
    })
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    format: cpal::SampleFormat,
    producer: Producer<f32>,
    error_sender: Sender<cpal::StreamError>,
) -> Result<cpal::Stream, String> {
    macro_rules! build_for_sample {
        ($sample:ty) => {
            build_input_stream::<$sample>(device, config, producer, error_sender)
        };
    }

    match format {
        cpal::SampleFormat::F32 => build_for_sample!(f32),
        cpal::SampleFormat::F64 => build_for_sample!(f64),
        cpal::SampleFormat::I8 => build_for_sample!(i8),
        cpal::SampleFormat::I16 => build_for_sample!(i16),
        cpal::SampleFormat::I24 => build_for_sample!(cpal::I24),
        cpal::SampleFormat::I32 => build_for_sample!(i32),
        cpal::SampleFormat::I64 => build_for_sample!(i64),
        cpal::SampleFormat::U8 => build_for_sample!(u8),
        cpal::SampleFormat::U16 => build_for_sample!(u16),
        cpal::SampleFormat::U24 => build_for_sample!(cpal::U24),
        cpal::SampleFormat::U32 => build_for_sample!(u32),
        cpal::SampleFormat::U64 => build_for_sample!(u64),
        _ => Err(format!("Unsupported microphone sample format: {format:?}")),
    }
}

fn build_input_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut producer: Producer<f32>,
    error_sender: Sender<cpal::StreamError>,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    device
        .build_input_stream::<T, _, _>(
            config,
            move |data: &[T], _| {
                for sample in data {
                    let _ = producer.push(f32::from_sample(*sample));
                }
            },
            move |error| {
                let _ = error_sender.try_send(error);
            },
            // Avoid turning a finite ALSA poll timeout into a fatal stream error.
            None,
        )
        .map_err(|error| error.to_string())
}

fn stop_requested(receiver: &Receiver<()>) -> bool {
    matches!(
        receiver.try_recv(),
        Ok(()) | Err(TryRecvError::Disconnected)
    )
}

struct AudioPresenceGate {
    levels: VecDeque<f32>,
    noise_floor: f32,
    noise_floor_calibrated: bool,
    smoothed_level: f32,
    below_threshold_frames: usize,
    active: bool,
}

impl Default for AudioPresenceGate {
    fn default() -> Self {
        Self {
            levels: VecDeque::with_capacity(VOLUME_HISTORY_WINDOWS),
            noise_floor: MIN_ACTIVE_RMS,
            noise_floor_calibrated: false,
            smoothed_level: 0.0,
            below_threshold_frames: 0,
            active: false,
        }
    }
}

impl AudioPresenceGate {
    fn is_active(&self) -> bool {
        self.active
    }

    fn confirm_audio(&mut self) {
        self.active = true;
        self.below_threshold_frames = 0;
    }

    fn push_rms(&mut self, rms: f32) {
        self.smoothed_level += AUDIO_LEVEL_SMOOTHING * (rms - self.smoothed_level);
        self.levels.push_back(rms);
        if self.levels.len() > VOLUME_HISTORY_WINDOWS {
            self.levels.pop_front();
        }

        if !self.active && self.levels.len() >= VOLUME_CALIBRATION_WINDOWS {
            self.update_noise_floor();
        }

        let open_threshold =
            (self.noise_floor * VOLUME_GATE_OPEN_RATIO).max(VOLUME_GATE_MIN_OPEN_RMS);
        let close_threshold =
            (self.noise_floor * VOLUME_GATE_CLOSE_RATIO).max(VOLUME_GATE_MIN_CLOSE_RMS);
        if self.active {
            if self.smoothed_level < close_threshold {
                self.below_threshold_frames = self.below_threshold_frames.saturating_add(1);
                if self.below_threshold_frames >= VOLUME_GATE_CLOSE_DELAY_FRAMES {
                    self.active = false;
                    self.below_threshold_frames = 0;
                }
            } else {
                self.below_threshold_frames = 0;
            }
        } else if self.levels.len() >= VOLUME_CALIBRATION_WINDOWS
            && self.smoothed_level >= open_threshold
        {
            self.active = true;
            self.below_threshold_frames = 0;
        }
    }

    fn update_noise_floor(&mut self) {
        let mut levels: Vec<_> = self.levels.iter().copied().collect();
        levels.sort_by(f32::total_cmp);
        let lower_quintile = levels[(levels.len() - 1) / 5];
        if !self.noise_floor_calibrated {
            self.noise_floor = lower_quintile;
            self.noise_floor_calibrated = true;
        } else {
            let alpha = if lower_quintile < self.noise_floor {
                NOISE_FLOOR_FALL_ALPHA
            } else {
                NOISE_FLOOR_RISE_ALPHA
            };
            self.noise_floor += alpha * (lower_quintile - self.noise_floor);
        }
    }
}

fn push_recent_bpm(values: &mut VecDeque<f32>, bpm: f32) {
    values.push_back(bpm);
    if values.len() > BPM_HISTORY_LENGTH {
        values.pop_front();
    }
}

fn mean_recent_bpm(values: &VecDeque<f32>) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f32>() / values.len() as f32)
}

fn select_tempo(
    tracker: &BpmTracker,
    volume_gate: &AudioPresenceGate,
    recent_bpms: &VecDeque<f32>,
) -> (Option<f32>, bool) {
    if !volume_gate.is_active() {
        return (None, false);
    }
    if let Some(bpm) = tracker.bpm {
        return (Some(bpm), false);
    }
    (mean_recent_bpm(recent_bpms), true)
}

#[derive(Clone, Copy, Debug)]
enum TrackerUpdate {
    Locked(f32),
    LostLock,
}

#[derive(Default)]
struct BpmTracker {
    onset_history: VecDeque<f32>,
    previous_rms: f32,
    frames_since_reset: usize,
    quiet_frames: usize,
    last_onset_frame: Option<usize>,
    low_confidence_windows: usize,
    candidate_bpm: Option<f32>,
    candidate_windows: usize,
    unstable_candidate_windows: usize,
    bpm: Option<f32>,
}

impl BpmTracker {
    fn push_rms(&mut self, rms: f32) -> Option<TrackerUpdate> {
        self.frames_since_reset += 1;
        self.quiet_frames = if rms < MIN_ACTIVE_RMS {
            self.quiet_frames + 1
        } else {
            0
        };

        let rise = (rms - self.previous_rms).max(0.0);
        let onset = if rise >= MIN_ONSET_RISE.max(self.previous_rms * RELATIVE_ONSET_RISE) {
            self.last_onset_frame = Some(self.frames_since_reset);
            rise
        } else {
            0.0
        };
        self.previous_rms = rms;
        self.onset_history.push_back(onset);
        if self.onset_history.len() > RMS_HISTORY_SECONDS * RMS_WINDOWS_PER_SECOND {
            self.onset_history.pop_front();
        }

        let stale_onsets = self.last_onset_frame.map_or(
            self.frames_since_reset >= NO_BEAT_TIMEOUT_SECONDS * RMS_WINDOWS_PER_SECOND,
            |last_onset| {
                self.frames_since_reset.saturating_sub(last_onset)
                    >= NO_BEAT_TIMEOUT_SECONDS * RMS_WINDOWS_PER_SECOND
            },
        );
        let silence = self.quiet_frames >= SILENCE_TIMEOUT_SECONDS * RMS_WINDOWS_PER_SECOND;
        if stale_onsets || silence {
            let lost_lock = self.bpm.take().is_some();
            self.reset(rms);
            return lost_lock.then_some(TrackerUpdate::LostLock);
        }

        if !self
            .frames_since_reset
            .is_multiple_of(ESTIMATE_INTERVAL_SECONDS * RMS_WINDOWS_PER_SECOND)
            || self.onset_history.len() < MIN_ANALYSIS_SECONDS * RMS_WINDOWS_PER_SECOND
        {
            return None;
        }

        let Some((candidate_bpm, confidence)) = estimate_bpm(&self.onset_history) else {
            return self.record_low_confidence(rms);
        };
        if confidence < MIN_CONFIDENCE {
            return self.record_low_confidence(rms);
        }
        self.low_confidence_windows = 0;
        if let Some(previous) = self.candidate_bpm
            && bpm_candidates_match(previous, candidate_bpm)
        {
            self.candidate_bpm = Some(previous + BPM_SMOOTHING * (candidate_bpm - previous));
            self.candidate_windows = self.candidate_windows.saturating_add(1);
            self.unstable_candidate_windows = 0;
        } else {
            self.candidate_bpm = Some(candidate_bpm);
            self.candidate_windows = 1;
            if self.bpm.is_some() {
                self.unstable_candidate_windows = self.unstable_candidate_windows.saturating_add(1);
                if self.unstable_candidate_windows >= MAX_UNSTABLE_CANDIDATE_WINDOWS {
                    self.reset(rms);
                    return Some(TrackerUpdate::LostLock);
                }
            }
        }
        if self.candidate_windows < LOCK_CONFIRMATION_WINDOWS {
            return None;
        }

        let confirmed_bpm = self.candidate_bpm.expect("confirmed candidate has a BPM");
        let bpm = self.bpm.map_or(confirmed_bpm, |previous| {
            previous + BPM_SMOOTHING * (confirmed_bpm - previous)
        });
        self.bpm = Some(bpm);
        Some(TrackerUpdate::Locked(bpm))
    }

    fn record_low_confidence(&mut self, rms: f32) -> Option<TrackerUpdate> {
        self.candidate_bpm = None;
        self.candidate_windows = 0;
        self.unstable_candidate_windows = 0;
        self.low_confidence_windows = self
            .low_confidence_windows
            .saturating_add(1)
            .min(MAX_LOW_CONFIDENCE_WINDOWS);
        if self.low_confidence_windows < MAX_LOW_CONFIDENCE_WINDOWS || self.bpm.is_none() {
            return None;
        }

        self.reset(rms);
        Some(TrackerUpdate::LostLock)
    }

    fn reset(&mut self, rms: f32) {
        self.onset_history.clear();
        self.previous_rms = rms;
        self.frames_since_reset = 0;
        self.quiet_frames = 0;
        self.last_onset_frame = None;
        self.low_confidence_windows = 0;
        self.candidate_bpm = None;
        self.candidate_windows = 0;
        self.unstable_candidate_windows = 0;
        self.bpm = None;
    }
}

fn bpm_candidates_match(previous: f32, candidate: f32) -> bool {
    (previous - candidate).abs() <= (previous * BPM_CANDIDATE_TOLERANCE).max(3.0)
}

fn estimate_bpm(onset_history: &VecDeque<f32>) -> Option<(f32, f32)> {
    let minimum_history = MIN_ANALYSIS_SECONDS * RMS_WINDOWS_PER_SECOND;
    if onset_history.len() < minimum_history {
        return None;
    }

    if onset_history.iter().filter(|onset| **onset > 0.0).count() < MIN_ONSETS_FOR_ESTIMATE {
        return None;
    }

    let onset_history: Vec<_> = onset_history.iter().copied().collect();
    let min_lag = (RMS_WINDOWS_PER_SECOND as f32 * 60.0 / MAX_BPM).ceil() as usize;
    let max_lag = (RMS_WINDOWS_PER_SECOND as f32 * 60.0 / MIN_BPM).floor() as usize;
    let mut best: Option<(usize, f32)> = None;

    for lag in min_lag..=max_lag {
        let score = normalized_autocorrelation(&onset_history, lag);
        if best.is_none_or(|(_, best_score)| score > best_score) {
            best = Some((lag, score));
        }
    }

    let (lag, confidence) = best?;
    Some((
        60.0 * RMS_WINDOWS_PER_SECOND as f32 / lag as f32,
        confidence,
    ))
}

fn normalized_autocorrelation(values: &[f32], lag: usize) -> f32 {
    let overlap = values.len().saturating_sub(lag);
    if overlap < lag {
        return 0.0;
    }

    let left = &values[lag..];
    let right = &values[..overlap];
    let mean_left = left.iter().map(|value| f64::from(*value)).sum::<f64>() / overlap as f64;
    let mean_right = right.iter().map(|value| f64::from(*value)).sum::<f64>() / overlap as f64;
    let mut covariance = 0.0_f64;
    let mut left_energy = 0.0_f64;
    let mut right_energy = 0.0_f64;

    for (left, right) in left.iter().zip(right) {
        let left = f64::from(*left) - mean_left;
        let right = f64::from(*right) - mean_right;
        covariance += left * right;
        left_energy += left * left;
        right_energy += right * right;
    }

    let normalizer = (left_energy * right_energy).sqrt();
    if normalizer <= f64::EPSILON {
        0.0
    } else {
        (covariance / normalizer).clamp(-1.0, 1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_bpm_parser_accepts_finite_reel_tempos_only() {
        assert_eq!(parse_manual_bpm("145"), Some(145.0));
        assert_eq!(parse_manual_bpm("30"), Some(30.0));
        assert_eq!(parse_manual_bpm("300"), Some(300.0));
        assert_eq!(parse_manual_bpm("29"), None);
        assert_eq!(parse_manual_bpm("301"), None);
        assert_eq!(parse_manual_bpm("NaN"), None);
        assert_eq!(parse_manual_bpm("fast"), None);
    }

    #[test]
    fn adaptive_volume_gate_distinguishes_background_drop_and_silence() {
        let mut gate = AudioPresenceGate::default();
        for _ in 0..VOLUME_HISTORY_WINDOWS * 2 {
            gate.push_rms(0.01);
        }
        assert!(!gate.is_active());

        for _ in 0..RMS_WINDOWS_PER_SECOND {
            gate.push_rms(0.06);
        }
        assert!(gate.is_active());

        for _ in 0..RMS_WINDOWS_PER_SECOND {
            gate.push_rms(0.02);
        }
        assert!(
            gate.is_active(),
            "a quieter song section should keep the gate open"
        );

        for _ in 0..VOLUME_GATE_CLOSE_DELAY_FRAMES + RMS_WINDOWS_PER_SECOND / 2 {
            gate.push_rms(0.01);
        }
        assert!(
            !gate.is_active(),
            "sustained background level should close the gate"
        );
    }

    #[test]
    fn active_audio_uses_recent_bpm_mean_when_the_tracker_loses_lock() {
        let mut gate = AudioPresenceGate::default();
        gate.confirm_audio();
        let mut tracker = BpmTracker::default();
        let recent_bpms = VecDeque::from([100.0, 102.0, 98.0, 101.0, 99.0]);

        assert_eq!(
            select_tempo(&tracker, &gate, &recent_bpms),
            (Some(100.0), true)
        );

        tracker.bpm = Some(104.0);
        assert_eq!(
            select_tempo(&tracker, &gate, &recent_bpms),
            (Some(104.0), false)
        );

        gate.active = false;
        assert_eq!(select_tempo(&tracker, &gate, &recent_bpms), (None, false));
    }

    #[test]
    fn bpm_fallback_uses_the_mean_of_the_five_most_recent_values() {
        let mut values = VecDeque::new();
        for bpm in [60.0, 70.0, 80.0, 90.0, 100.0, 110.0] {
            push_recent_bpm(&mut values, bpm);
        }

        assert_eq!(values.len(), BPM_HISTORY_LENGTH);
        assert_eq!(mean_recent_bpm(&values), Some(90.0));
    }

    #[test]
    fn periodic_clicks_lock_to_their_tempo() {
        let mut tracker = BpmTracker::default();
        let mut estimate = None;

        for frame in 0..RMS_WINDOWS_PER_SECOND * 10 {
            let rms = if frame % 29 < 2 { 0.5 } else { 0.1 };
            if let Some(TrackerUpdate::Locked(bpm)) = tracker.push_rms(rms) {
                estimate = Some(bpm);
            }
        }

        let bpm = estimate.expect("a steady click track should produce a lock");
        assert!((bpm - 60.0 * RMS_WINDOWS_PER_SECOND as f32 / 29.0).abs() < 2.0);
    }

    #[test]
    fn tempo_locks_only_after_repeated_beat_evidence() {
        for (period_frames, expected_bpm, deadline_frames) in [(25, 120.0, 160), (50, 60.0, 260)] {
            let mut tracker = BpmTracker::default();
            let mut first_lock_frame = None;
            let mut last_bpm = None;

            for frame in 0..RMS_WINDOWS_PER_SECOND * 6 {
                let rms = if frame % period_frames < 2 { 0.5 } else { 0.1 };
                if let Some(TrackerUpdate::Locked(bpm)) = tracker.push_rms(rms) {
                    first_lock_frame.get_or_insert(frame);
                    last_bpm = Some(bpm);
                }
            }

            assert!(first_lock_frame.is_some_and(|frame| frame <= deadline_frames));
            assert!((last_bpm.unwrap() - expected_bpm).abs() < 4.0);
        }
    }

    #[test]
    fn two_transient_noise_spikes_do_not_produce_a_tempo_lock() {
        let mut tracker = BpmTracker::default();

        for frame in 0..RMS_WINDOWS_PER_SECOND * 5 {
            let rms = if frame == 0 || frame == 20 { 0.5 } else { 0.01 };
            assert!(!matches!(
                tracker.push_rms(rms),
                Some(TrackerUpdate::Locked(_))
            ));
        }
    }

    #[test]
    fn random_ambient_noise_does_not_produce_a_tempo_lock() {
        let mut tracker = BpmTracker::default();
        let mut random = 1_u32;

        for _ in 0..RMS_WINDOWS_PER_SECOND * 20 {
            random = random.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = (random >> 8) as f32 / 16_777_215.0;
            let rms = 0.01 + noise * 0.02;
            assert!(!matches!(
                tracker.push_rms(rms),
                Some(TrackerUpdate::Locked(_))
            ));
        }
    }

    #[test]
    fn steady_ambient_level_does_not_produce_a_tempo_lock() {
        let mut tracker = BpmTracker::default();
        for _ in 0..RMS_WINDOWS_PER_SECOND * 12 {
            assert!(tracker.push_rms(0.1).is_none());
        }
    }

    #[test]
    fn quiet_room_noise_drops_a_lock_within_the_no_beat_timeout() {
        let mut tracker = BpmTracker::default();
        for frame in 0..RMS_WINDOWS_PER_SECOND * 10 {
            let rms = if frame % 25 < 2 { 0.5 } else { 0.1 };
            tracker.push_rms(rms);
        }
        assert!(tracker.bpm.is_some());

        let mut lost_frame = None;
        for frame in 0..RMS_WINDOWS_PER_SECOND * 4 {
            let small_variation = ((frame * 37) % 11) as f32 * 0.00002;
            if matches!(
                tracker.push_rms(0.01 + small_variation),
                Some(TrackerUpdate::LostLock)
            ) {
                lost_frame = Some(frame);
                break;
            }
        }

        assert!(lost_frame.is_some_and(|frame| {
            frame <= NO_BEAT_TIMEOUT_SECONDS * RMS_WINDOWS_PER_SECOND + 30
        }));
        assert!(tracker.bpm.is_none());
    }

    #[test]
    fn irregular_onsets_drop_an_existing_tempo_lock() {
        let mut tracker = BpmTracker::default();
        for frame in 0..RMS_WINDOWS_PER_SECOND * 10 {
            let rms = if frame % 25 < 2 { 0.5 } else { 0.1 };
            tracker.push_rms(rms);
        }
        assert!(tracker.bpm.is_some());

        let intervals = [23, 31, 18, 42, 27, 35, 19, 38, 24, 31, 17, 45];
        let mut lost_lock = false;
        for interval in intervals.into_iter().cycle().take(40) {
            for frame in 0..interval {
                let rms = if frame == 0 { 0.5 } else { 0.1 };
                lost_lock |= matches!(tracker.push_rms(rms), Some(TrackerUpdate::LostLock));
            }
        }
        assert!(lost_lock);
        assert!(tracker.bpm.is_none());
    }

    #[test]
    fn silence_drops_an_existing_tempo_lock() {
        let mut tracker = BpmTracker::default();
        let mut locked = false;
        for frame in 0..RMS_WINDOWS_PER_SECOND * 10 {
            let rms = if frame % 25 == 0 { 0.5 } else { 0.1 };
            locked |= matches!(tracker.push_rms(rms), Some(TrackerUpdate::Locked(_)));
        }
        assert!(locked);

        let mut lost = false;
        for _ in 0..RMS_WINDOWS_PER_SECOND * SILENCE_TIMEOUT_SECONDS {
            lost |= matches!(tracker.push_rms(0.0), Some(TrackerUpdate::LostLock));
        }
        assert!(lost);
        assert!(tracker.bpm.is_none());
    }
}
