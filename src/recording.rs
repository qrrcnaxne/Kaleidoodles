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

/// Configuration for deterministic frame-capture export.
///
/// Created from CLI args (`--record <dir>`) in `app::run` and consumed by the
/// active sketch to render a fixed-duration, fixed-framerate clip.
#[derive(Resource, Debug)]
pub struct Recording {
    pub output_dir: PathBuf,
    pub width: u32,
    pub height: u32,
    pub max_frames: u32,
    pub fixed_dt: f32,
    pub saved_frames: u32,
}

impl Recording {
    /// Looks for `--record <directory>` in the process arguments.
    pub fn from_args() -> Option<Self> {
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            if arg == "--record" {
                let dir = args.next().unwrap_or_else(|| "frames".to_string());
                return Some(Self::new(dir, RECORDING_WIDTH, RECORDING_HEIGHT));
            }
        }
        None
    }

    pub fn new(output_dir: impl Into<PathBuf>, width: u32, height: u32) -> Self {
        let max_frames = (RECORDING_FPS * RECORDING_DURATION_SECONDS).max(0.0) as u32;
        Self {
            output_dir: output_dir.into(),
            width,
            height,
            max_frames,
            fixed_dt: 1.0 / RECORDING_FPS,
            saved_frames: 0,
        }
    }
}

/// Handle to the offscreen render target used while recording.
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
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let handle = images.add(image);

    commands.spawn(ImageCopier::new(handle.clone(), size, &render_device));
    commands.insert_resource(RecordingRenderTarget(handle));
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

    let mut rgba = Vec::with_capacity(data.row_bytes * data.height as usize);
    for row in 0..data.height as usize {
        let start = row * data.padded_row_bytes;
        rgba.extend_from_slice(&data.bytes[start..start + data.row_bytes]);
    }

    let img =
        image::RgbaImage::from_raw(data.width, data.height, rgba).expect("invalid RGBA buffer");
    let frame_path = recording
        .output_dir
        .join(format!("{:04}.png", recording.saved_frames));
    img.save(&frame_path)
        .unwrap_or_else(|e| panic!("failed to save {frame_path:?}: {e}"));

    recording.saved_frames += 1;
    if recording.saved_frames >= recording.max_frames {
        app_exit.write(AppExit::Success);
    }
}
