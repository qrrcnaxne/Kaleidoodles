//! CPU Gray–Scott reaction–diffusion on a periodic portrait grid.

use std::f32::consts::TAU;

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::PrimaryWindow,
};

use super::common::{halton, simulation_delta, spawn_sketch_camera, viewport_size};
use crate::recording::{Recording, RecordingRenderTarget};

const WIDTH: usize = 180;
const HEIGHT: usize = 320;
// MIT's Gray–Scott examples identify this as the dynamic "wandering bubbles" regime.
const FEED: f32 = 0.012;
const KILL: f32 = 0.05;
const STEPS_PER_SECOND: f32 = 200.0;
const WARMUP_STEPS: usize = 800;

pub(super) struct ReactionDiffusionPlugin;

impl Plugin for ReactionDiffusionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup)
            .add_systems(Update, (advance, fit_canvas).chain());
    }
}

#[derive(Resource)]
struct ChemicalField {
    cells: Vec<Vec2>,
    next: Vec<Vec2>,
    image: Handle<Image>,
    pending_steps: f32,
    elapsed: f32,
    paused: bool,
}

impl ChemicalField {
    fn new(image: Handle<Image>) -> Self {
        let mut field = Self {
            cells: vec![Vec2::new(1.0, 0.0); WIDTH * HEIGHT],
            next: vec![Vec2::ZERO; WIDTH * HEIGHT],
            image,
            pending_steps: 0.0,
            elapsed: 0.0,
            paused: false,
        };
        field.reset();
        field
    }

    fn reset(&mut self) {
        self.cells.fill(Vec2::new(1.0, 0.0));
        for seed in 1..=48 {
            let cx = (halton(seed, 2) * WIDTH as f32) as usize;
            let cy = (halton(seed, 3) * HEIGHT as f32) as usize;
            for dy in -4_i32..=4 {
                for dx in -4_i32..=4 {
                    if dx * dx + dy * dy <= 16 {
                        let x = (cx as i32 + dx).rem_euclid(WIDTH as i32) as usize;
                        let y = (cy as i32 + dy).rem_euclid(HEIGHT as i32) as usize;
                        self.cells[y * WIDTH + x] = Vec2::new(0.5, 0.25);
                    }
                }
            }
        }
        for _ in 0..WARMUP_STEPS {
            self.step();
        }
        self.pending_steps = 0.0;
        self.elapsed = 0.0;
        self.paused = false;
    }

    fn step(&mut self) {
        for y in 0..HEIGHT {
            let above = (y + HEIGHT - 1) % HEIGHT;
            let below = (y + 1) % HEIGHT;
            for x in 0..WIDTH {
                let left = (x + WIDTH - 1) % WIDTH;
                let right = (x + 1) % WIDTH;
                let i = y * WIDTH + x;
                let cell = self.cells[i];
                // Nine-point stencil: center -1, axial .2, diagonal .05.
                let lap = -cell
                    + 0.2
                        * (self.cells[y * WIDTH + left]
                            + self.cells[y * WIDTH + right]
                            + self.cells[above * WIDTH + x]
                            + self.cells[below * WIDTH + x])
                    + 0.05
                        * (self.cells[above * WIDTH + left]
                            + self.cells[above * WIDTH + right]
                            + self.cells[below * WIDTH + left]
                            + self.cells[below * WIDTH + right]);
                let reaction = cell.x * cell.y * cell.y;
                let du = lap.x - reaction + FEED * (1.0 - cell.x);
                let dv = 0.5 * lap.y + reaction - (FEED + KILL) * cell.y;
                // Forward Euler, dt=1; clipping protects the concentration range.
                self.next[i] = (cell + Vec2::new(du, dv)).clamp(Vec2::ZERO, Vec2::ONE);
            }
        }
        std::mem::swap(&mut self.cells, &mut self.next);
    }

    fn paint(&self, image: &mut Image) {
        let Some(pixels) = image.data.as_mut() else {
            return;
        };
        for (cell, pixel) in self.cells.iter().zip(pixels.chunks_exact_mut(4)) {
            let strength = (cell.y * 3.2).clamp(0.0, 1.0);
            let phase = strength * 0.85 + self.elapsed * 0.018;
            for (channel, offset) in pixel[..3].iter_mut().zip([0.0, 0.33, 0.67]) {
                let color = 0.5 + 0.5 * (TAU * (phase + offset)).cos();
                *channel = ((0.012 + strength.sqrt() * color * 0.97) * 255.0) as u8;
            }
            pixel[3] = 255;
        }
    }
}

#[derive(Component)]
struct Canvas;

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    render_target: Option<Res<RecordingRenderTarget>>,
    mut window: Option<Single<&mut Window, With<PrimaryWindow>>>,
) {
    if let Some(window) = window.as_mut() {
        window.title = "Reaction–diffusion • R restart • Space pause".into();
        window.resolution.set(540.0, 960.0);
    }
    spawn_sketch_camera(&mut commands, render_target.as_deref());
    let mut image = Image::new_fill(
        Extent3d {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let mut field = ChemicalField::new(Handle::default());
    field.paint(&mut image);
    let handle = images.add(image);
    field.image = handle.clone();
    commands.spawn((Sprite::from_image(handle), Canvas));
    commands.insert_resource(field);
}

fn advance(
    time: Res<Time>,
    recording: Option<Res<Recording>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut field: ResMut<ChemicalField>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut restarted = false;
    if recording.is_none()
        && let Some(keys) = keys
    {
        if keys.just_pressed(KeyCode::KeyR) {
            field.reset();
            restarted = true;
        }
        if keys.just_pressed(KeyCode::Space) {
            field.paused = !field.paused;
        }
    }
    if !field.paused && !restarted {
        let dt = simulation_delta(recording.as_deref(), time.delta_secs().min(0.1));
        field.elapsed += dt;
        field.pending_steps += dt * STEPS_PER_SECOND;
        while field.pending_steps >= 1.0 {
            field.step();
            field.pending_steps -= 1.0;
        }
    }
    if let Some(mut image) = images.get_mut(&field.image) {
        field.paint(&mut image);
    }
}

fn fit_canvas(
    recording: Option<Res<Recording>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    mut canvas: Single<&mut Sprite, With<Canvas>>,
) {
    let size = window.as_ref().map(|w| Vec2::new(w.width(), w.height()));
    if let Some(viewport) = viewport_size(recording.as_deref(), size) {
        let scale = (viewport.x / WIDTH as f32).min(viewport.y / HEIGHT as f32);
        canvas.custom_size = Some(Vec2::new(WIDTH as f32, HEIGHT as f32) * scale);
    }
}
