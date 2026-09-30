use crate::mesh::{MeshBuilder, transform_mesh};
use macroquad::prelude::*;
use std::cell::OnceCell;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PickupKind {
    Health,
    Ammo,
    Armor,
}

impl PickupKind {
    pub fn color(self) -> Color {
        match self {
            PickupKind::Health => Color::new(0.86, 0.86, 0.90, 1.0),
            PickupKind::Ammo => Color::new(0.62, 0.52, 0.24, 1.0),
            PickupKind::Armor => Color::new(0.26, 0.48, 0.78, 1.0),
        }
    }

    pub fn accent(self) -> Color {
        match self {
            PickupKind::Health => Color::new(0.85, 0.16, 0.16, 1.0),
            PickupKind::Ammo => Color::new(0.95, 0.78, 0.30, 1.0),
            PickupKind::Armor => Color::new(0.55, 0.88, 1.00, 1.0),
        }
    }

    pub fn max_amount(self) -> f32 {
        match self {
            PickupKind::Health => 35.0,
            PickupKind::Ammo => 60.0,
            PickupKind::Armor => 50.0,
        }
    }

    /// Index into the cached prototype table.
    fn index(self) -> usize {
        match self {
            PickupKind::Health => 0,
            PickupKind::Ammo => 1,
            PickupKind::Armor => 2,
        }
    }
}

thread_local! {
    /// The three prototypes, built on first use. The geometry depends only on the
    /// kind, so rebuilding it every frame — which is what `mesh` used to do — was
    /// rebuilding the same three meshes forever. `Mesh` holds a texture handle,
    /// so this cannot be a plain `static`, and macroquad draws from one thread,
    /// so a thread-local is the whole of the sharing problem.
    static PROTOS: OnceCell<[Mesh; 3]> = const { OnceCell::new() };
}

pub struct Pickup {
    pub pos: Vec3,
    pub kind: PickupKind,
    pub amount: f32,
    spin: f32,
    phase: f32,
}

impl Pickup {
    pub fn new(pos: Vec3, kind: PickupKind, amount: f32) -> Self {
        Self {
            pos,
            kind,
            amount: amount.min(kind.max_amount()),
            spin: 0.0,
            phase: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.spin = (self.spin + dt * 1.5) % std::f32::consts::TAU;
        self.phase += dt;
    }

    pub fn centre(&self) -> Vec3 {
        self.pos + vec3(0.0, 0.55 + (self.phase * 2.0).sin() * 0.07, 0.0)
    }

    pub fn mesh(&self) -> Mesh {
        PROTOS.with(|cell| {
            let protos = cell.get_or_init(|| {
                [
                    proto(PickupKind::Health),
                    proto(PickupKind::Ammo),
                    proto(PickupKind::Armor),
                ]
            });
            let proto = &protos[self.kind.index()];
            let pulse = 0.85 + (self.phase * 4.0).sin() * 0.15;
            let mat = Mat4::from_translation(self.centre()) * Mat4::from_rotation_y(self.spin);
            let mut m = transform_mesh(proto, mat);
            // Gentle emissive pulse so pickups read clearly against dark geometry.
            for v in &mut m.vertices {
                v.color = [
                    (v.color[0] as f32 * pulse).min(255.0) as u8,
                    (v.color[1] as f32 * pulse).min(255.0) as u8,
                    (v.color[2] as f32 * pulse).min(255.0) as u8,
                    v.color[3],
                ];
            }
            m
        })
    }
}

/// Builds one kind's geometry. Called once per kind, to fill `PROTOS`.
fn proto(kind: PickupKind) -> Mesh {
    let mut mb = MeshBuilder::new();
    let body = kind.color();
    let accent = kind.accent();
    match kind {
        PickupKind::Health => {
            mb.add_box(vec3(-0.26, -0.20, -0.20), vec3(0.26, 0.20, 0.20), body);
            // red cross, on two faces
            mb.add_box(vec3(-0.06, -0.15, -0.215), vec3(0.06, 0.15, -0.195), accent);
            mb.add_box(vec3(-0.15, -0.06, -0.215), vec3(0.15, 0.06, -0.195), accent);
            mb.add_box(vec3(-0.06, -0.15, 0.195), vec3(0.06, 0.15, 0.215), accent);
            mb.add_box(vec3(-0.15, -0.06, 0.195), vec3(0.15, 0.06, 0.215), accent);
        }
        PickupKind::Ammo => {
            mb.add_box(vec3(-0.30, -0.18, -0.20), vec3(0.30, 0.18, 0.20), body);
            mb.add_box(vec3(-0.30, 0.05, -0.21), vec3(0.30, 0.18, 0.21), accent);
            mb.add_box(vec3(-0.06, -0.18, -0.21), vec3(0.06, 0.18, 0.21), accent);
        }
        PickupKind::Armor => {
            mb.add_box(vec3(-0.24, -0.22, -0.16), vec3(0.24, 0.22, 0.16), body);
            mb.add_box(vec3(-0.24, 0.12, -0.17), vec3(0.24, 0.22, 0.17), accent);
            mb.add_box(vec3(-0.10, -0.22, -0.17), vec3(0.10, -0.06, 0.17), accent);
        }
    }
    mb.finish()
}
