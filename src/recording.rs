use std::path::PathBuf;

use bevy::{
    app::{AppExit, Plugin},
    asset::Handle,
    ecs::component::Component,
    image::{Image, TextureFormatPixelInfo},
    prelude::*,
    render::{
        Extract, Render, RenderApp, RenderSystems,
        render_asset::RenderAssets,
        render_resource::{
            Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, MapMode,
            PollType, TexelCopyBufferInfo, TexelCopyBufferLayout, TextureFormat, TextureUsages,
        },
        renderer::{RenderContext, RenderDevice, RenderGraph, RenderQueue},
    },
};
use crossbeam_channel::{Receiver, Sender, bounded};

pub const RECORDING_WIDTH: u32 = 1080;
pub const RECORDING_HEIGHT: u32 = 1920;
pub const RECORDING_FPS: f32 = 60.0;
pub const RECORDING_DURATION_SECONDS: f32 = 10.0;
/// Renders to discard before saving: the offscreen target and its meshes and
/// materials need a few frames to reach the GPU, so early captures are blank.
const WARMUP_FRAMES: u32 = 8;

/// Fixed-step headless run configuration for frame export or population-only simulation.
///
/// Created from CLI args (`--record <dir>` or `--simulate`) in `app::run` and
/// consumed by the active sketch to use fixed dimensions and a fixed timestep.
#[derive(Resource, Debug)]
pub struct Recording {
    pub output_dir: PathBuf,
    pub width: u32,
    pub height: u32,
    pub max_frames: u32,
    pub fixed_dt: f32,
    pub completed_frames: u32,
    warmup_frames: u32,
    capture_frames: bool,
}

impl Recording {
    /// Looks for `--record <directory>` or `--simulate`, plus optional
    /// `--fps <n>` and `--duration <seconds>` flags in the process arguments.
    pub fn from_args() -> Option<Self> {
        let mut args = std::env::args().skip(1);
        let mut fps = None;
        let mut duration = None;
        let mut output_dir = None;
        let mut simulate_only = false;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--record" => {
                    output_dir = Some(args.next().unwrap_or_else(|| "frames".to_string()));
                }
                "--simulate" => simulate_only = true,
                "--fps" => fps = args.next().and_then(|value| value.parse().ok()),
                "--duration" => duration = args.next().and_then(|value| value.parse().ok()),
                _ => {}
            }
        }

        if simulate_only {
            Some(Self::simulation_only(
                RECORDING_WIDTH,
                RECORDING_HEIGHT,
                fps.unwrap_or(30.0),
                duration.unwrap_or(120.0),
            ))
        } else {
            output_dir.map(|dir| {
                Self::new(
                    dir,
                    RECORDING_WIDTH,
                    RECORDING_HEIGHT,
                    fps.unwrap_or(RECORDING_FPS),
                    duration.unwrap_or(RECORDING_DURATION_SECONDS),
                )
            })
        }
    }

    pub fn new(
        output_dir: impl Into<PathBuf>,
        width: u32,
        height: u32,
        fps: f32,
        duration_secs: f32,
    ) -> Self {
        Self {
            output_dir: output_dir.into(),
            width,
            height,
            max_frames: (fps * duration_secs).max(0.0) as u32,
            fixed_dt: 1.0 / fps,
            completed_frames: 0,
            warmup_frames: WARMUP_FRAMES,
            capture_frames: true,
        }
    }

    fn simulation_only(width: u32, height: u32, fps: f32, duration_secs: f32) -> Self {
        let mut run = Self::new(PathBuf::new(), width, height, fps, duration_secs);
        run.capture_frames = false;
        run.warmup_frames = 0;
        run
    }

    pub fn captures_frames(&self) -> bool {
        self.capture_frames
    }
}

/// Handle to the offscreen render target used during headless runs.
#[derive(Resource, Debug, Deref, Clone)]
pub struct RecordingRenderTarget(pub Handle<Image>);

pub struct RecordingPlugin;

impl Plugin for RecordingPlugin {
    fn build(&self, app: &mut App) {
        let (sender, receiver) = bounded::<FrameData>(1);
        app.insert_resource(MainWorldReceiver(receiver))
            .add_systems(Startup, setup_render_target)
            .add_systems(PostUpdate, save_frame);

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .insert_resource(RenderWorldSender(sender))
            .add_systems(ExtractSchedule, extract_image_copier)
            .add_systems(RenderGraph, copy_image_to_buffer)
            .add_systems(
                Render,
                receive_image_from_buffer.after(RenderSystems::Render),
            );
    }
}

/// Drives the same deterministic headless simulation without writing image files.
pub struct HeadlessSimulationPlugin;

impl Plugin for HeadlessSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_simulation_render_target)
            .add_systems(PostUpdate, advance_simulation);
    }
}

#[derive(Resource, Deref)]
struct MainWorldReceiver(Receiver<FrameData>);

#[derive(Resource, Deref)]
struct RenderWorldSender(Sender<FrameData>);

struct FrameData {
    width: u32,
    height: u32,
    row_bytes: usize,
    padded_row_bytes: usize,
    bytes: Vec<u8>,
}

fn setup_render_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    recording: Res<Recording>,
    render_device: Res<RenderDevice>,
) {
    let (handle, size) = create_render_target(&mut images, &recording, true);
    commands.spawn(ImageCopier::new(handle.clone(), size, &render_device));
    commands.insert_resource(RecordingRenderTarget(handle));
}

fn setup_simulation_render_target(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    recording: Res<Recording>,
) {
    let (handle, _) = create_render_target(&mut images, &recording, false);
    commands.insert_resource(RecordingRenderTarget(handle));
}

fn create_render_target(
    images: &mut Assets<Image>,
    recording: &Recording,
    copy_source: bool,
) -> (Handle<Image>, Extent3d) {
    let size = Extent3d {
        width: recording.width,
        height: recording.height,
        ..default()
    };
    let mut image = Image::new_target_texture(
        size.width,
        size.height,
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    );
    if copy_source {
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    }
    (images.add(image), size)
}

fn advance_simulation(mut recording: ResMut<Recording>, mut app_exit: MessageWriter<AppExit>) {
    recording.completed_frames += 1;
    if recording.completed_frames >= recording.max_frames {
        app_exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulation_run_has_fixed_rate_without_frame_capture() {
        let run = Recording::simulation_only(1080, 1920, 30.0, 120.0);
        assert!(!run.captures_frames());
        assert_eq!(run.max_frames, 3_600);
        assert_eq!(run.fixed_dt, 1.0 / 30.0);
        assert_eq!(run.warmup_frames, 0);
    }

    #[test]
    fn capture_run_saves_the_requested_frames_after_warmup() {
        let run = Recording::new("frames", 1080, 1920, 30.0, 180.0);
        assert!(run.captures_frames());
        assert_eq!(run.max_frames, 5_400);
        assert_eq!(run.warmup_frames, WARMUP_FRAMES);
    }
}

#[derive(Component, Clone)]
struct ImageCopier {
    buffer: Buffer,
    src_image: Handle<Image>,
    width: u32,
    height: u32,
    padded_row_bytes: usize,
}

impl ImageCopier {
    fn new(src_image: Handle<Image>, size: Extent3d, render_device: &RenderDevice) -> Self {
        let row_bytes = size.width as usize * TextureFormat::Rgba8Unorm.pixel_size().unwrap();
        let padded_row_bytes = RenderDevice::align_copy_bytes_per_row(row_bytes);
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("recording_readback_buffer"),
            size: (padded_row_bytes * size.height as usize) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            src_image,
            width: size.width,
            height: size.height,
            padded_row_bytes,
        }
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
struct ImageCopiers(Vec<ImageCopier>);

fn extract_image_copier(mut commands: Commands, query: Extract<Query<&ImageCopier>>) {
    commands.insert_resource(ImageCopiers(query.iter().cloned().collect()));
}

fn copy_image_to_buffer(
    render_context: RenderContext,
    image_copiers: Res<ImageCopiers>,
    render_queue: Res<RenderQueue>,
    gpu_images: Res<RenderAssets<bevy::render::texture::GpuImage>>,
) {
    for copier in image_copiers.iter() {
        let Some(gpu_image) = gpu_images.get(&copier.src_image) else {
            continue;
        };
        let mut encoder = render_context
            .render_device()
            .create_command_encoder(&CommandEncoderDescriptor::default());
        let block_dimensions = gpu_image.texture_descriptor.format.block_dimensions();
        let block_size = gpu_image
            .texture_descriptor
            .format
            .block_copy_size(None)
            .unwrap();
        let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
            (gpu_image.texture_descriptor.size.width as usize / block_dimensions.0 as usize)
                * block_size as usize,
        );
        encoder.copy_texture_to_buffer(
            gpu_image.texture.as_image_copy(),
            TexelCopyBufferInfo {
                buffer: &copier.buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(
                        std::num::NonZeroU32::new(padded_bytes_per_row as u32)
                            .unwrap()
                            .into(),
                    ),
                    rows_per_image: None,
                },
            },
            gpu_image.texture_descriptor.size,
        );
        render_queue.submit(std::iter::once(encoder.finish()));
    }
}

fn receive_image_from_buffer(
    image_copiers: Res<ImageCopiers>,
    render_device: Res<RenderDevice>,
    sender: Res<RenderWorldSender>,
) {
    for copier in image_copiers.iter() {
        let buffer_slice = copier.buffer.slice(..);
        let (s, r) = bounded::<()>(1);
        buffer_slice.map_async(MapMode::Read, move |result| {
            if result.is_ok() {
                let _ = s.send(());
            }
        });
        render_device
            .poll(PollType::wait_indefinitely())
            .expect("failed to poll render device");
        r.recv().expect("failed to receive buffer map notification");

        let row_bytes = copier.width as usize * TextureFormat::Rgba8Unorm.pixel_size().unwrap();
        let _ = sender.send(FrameData {
            width: copier.width,
            height: copier.height,
            row_bytes,
            padded_row_bytes: copier.padded_row_bytes,
            bytes: buffer_slice.get_mapped_range().to_vec(),
        });
        copier.buffer.unmap();
    }
}

fn save_frame(
    mut recording: ResMut<Recording>,
    receiver: Res<MainWorldReceiver>,
    mut app_exit: MessageWriter<AppExit>,
) {
    let Ok(data) = receiver.try_recv() else {
        return;
    };

    // Discard the first few rendered frames; the offscreen pipeline has not
    // drawn the scene yet, so saving them would put blank frames at the start.
    if recording.warmup_frames > 0 {
        recording.warmup_frames -= 1;
        return;
    }

    let mut rgba = Vec::with_capacity(data.row_bytes * data.height as usize);
    for row in 0..data.height as usize {
        let start = row * data.padded_row_bytes;
        rgba.extend_from_slice(&data.bytes[start..start + data.row_bytes]);
    }

    let img =
        image::RgbaImage::from_raw(data.width, data.height, rgba).expect("invalid RGBA buffer");
    let frame_path = recording
        .output_dir
        .join(format!("{:04}.png", recording.completed_frames));
    img.save(&frame_path)
        .unwrap_or_else(|e| panic!("failed to save {frame_path:?}: {e}"));

    recording.completed_frames += 1;
    if recording.completed_frames >= recording.max_frames {
        app_exit.write(AppExit::Success);
    }
}
