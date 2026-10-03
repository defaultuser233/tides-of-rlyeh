//! The six surfaces the world is dressed with, and the textures behind them.
//!
//! A `Mesh` carries exactly one texture (`models.rs:41-45`), so the surface is
//! also the unit of batching: `Level` keeps one mesh per surface and draws each
//! once. Everything else in the renderer is textureless.

use crate::assets::{Asset, bytes};
use macroquad::prelude::*;

/// One variant per PNG in `assets/textures/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Concrete,
    Tiles,
    MetalPlate,
    MetalRusty,
    Corrugated,
    Crate,
}

impl Surface {
    /// Draw order, and the index into `Textures` and `Level::meshes`.
    pub const ALL: [Surface; 6] = [
        Surface::Concrete,
        Surface::Tiles,
        Surface::MetalPlate,
        Surface::MetalRusty,
        Surface::Corrugated,
        Surface::Crate,
    ];

    pub fn index(self) -> usize {
        match self {
            Surface::Concrete => 0,
            Surface::Tiles => 1,
            Surface::MetalPlate => 2,
            Surface::MetalRusty => 3,
            Surface::Corrugated => 4,
            Surface::Crate => 5,
        }
    }

    /// The tile the level is built from.
    fn png(self) -> &'static [u8] {
        bytes(match self {
            Surface::Concrete => Asset::TexConcreteFloor,
            Surface::Tiles => Asset::TexConcreteTiles,
            Surface::MetalPlate => Asset::TexMetalPlate,
            Surface::MetalRusty => Asset::TexMetalRusty,
            Surface::Corrugated => Asset::TexMetalCorrugated,
            Surface::Crate => Asset::TexCrateSide,
        })
    }

    /// How many metres one copy of the texture covers.
    ///
    /// This is what stops a 20 m wall from being one stretched tile: the UVs
    /// are generated per face from its physical size, so the density is the
    /// same on a floor slab, a wall panel and a crate.
    pub fn metres_per_tile(self) -> f32 {
        match self {
            Surface::Concrete => 4.0,
            Surface::Tiles => 2.4,
            Surface::MetalPlate => 2.0,
            Surface::MetalRusty => 2.4,
            Surface::Corrugated => 1.6,
            Surface::Crate => 1.2,
        }
    }

    /// Mean RGB of the shipped texture, per channel and in 0..1.
    ///
    /// The 3D shader is `color * texture2D(Texture, uv)` (`quad_gl.rs:1060-1068`),
    /// so a block's palette colour is multiplied by the photo: a warm sand floor
    /// texture quietly turns a blue-grey arena brown, and every surface loses
    /// whatever the palette said about it. Dividing the texture's own mean back
    /// out, per channel, makes the average pixel come out at exactly the colour
    /// the palette asked for — the texture rides on top as detail rather than
    /// deciding the hue.
    ///
    /// These are measurements, not guesses: `the_mean_table_matches_the_textures`
    /// re-derives every one of them from the embedded PNG and fails if it drifts.
    pub const fn mean(self) -> [f32; 3] {
        match self {
            Surface::Concrete => [0.6501, 0.5243, 0.3985],
            Surface::Tiles => [0.4045, 0.3855, 0.3603],
            Surface::MetalPlate => [0.2397, 0.1946, 0.1087],
            Surface::MetalRusty => [0.6735, 0.5632, 0.3536],
            Surface::Corrugated => [0.6676, 0.7011, 0.6844],
            Surface::Crate => [0.3628, 0.5359, 0.3149],
        }
    }
}

/// A colour paired with the surface it dresses.
///
/// Every block in a map is built from one of these, so the material is named
/// once where the palette is defined instead of at each of the sixty-odd places
/// that colour is used.
#[derive(Clone, Copy)]
pub struct Skin {
    pub color: Color,
    pub surface: Surface,
}

pub const fn skin(r: f32, g: f32, b: f32, surface: Surface) -> Skin {
    Skin {
        color: Color::new(r, g, b, 1.0),
        surface,
    }
}

impl Skin {
    /// Scale the colour without touching the surface — the same ±6% wobble the
    /// maps used to apply, kept so large surfaces do not read as one flat slab
    /// even where the texture is uniform.
    pub fn vary(self, k: f32) -> Skin {
        Skin {
            color: Color::new(
                (self.color.r * k).min(1.0),
                (self.color.g * k).min(1.0),
                (self.color.b * k).min(1.0),
                self.color.a,
            ),
            ..self
        }
    }

    /// The colour to actually paint a face with, given the texture that will be
    /// multiplied into it.
    ///
    /// Dividing the texture's mean back out restores the colour the palette
    /// asks for. A colour brighter than the texture it sits on cannot be
    /// reached this way — a vertex colour cannot exceed 1 — so when a channel
    /// overflows every channel is scaled by the same amount instead. The trim
    /// stays amber and goes darker; clipping one channel instead would turn it
    /// white.
    pub fn uniform(self) -> Color {
        let mean = self.surface.mean();
        let (r, g, b) = (
            self.color.r / mean[0].max(1e-3),
            self.color.g / mean[1].max(1e-3),
            self.color.b / mean[2].max(1e-3),
        );
        let peak = r.max(g).max(b);
        if peak > 1.0 {
            Color::new(r / peak, g / peak, b / peak, self.color.a)
        } else {
            Color::new(r, g, b, self.color.a)
        }
    }
}

/// The loaded textures, indexed by `Surface::index`.
pub struct Textures([Texture2D; 6]);

impl Textures {
    /// Decodes and uploads the six embedded textures.
    ///
    /// Needs a live GL context, so it cannot happen in `Level::from_layout`
    /// (which the tests build headlessly).
    pub fn load() -> Self {
        let textures = Surface::ALL.map(|s| {
            let img = Image::from_file_with_format(s.png(), Some(ImageFormat::Png))
                .expect("a bundled surface texture is not a decodable PNG");
            let tex = Texture2D::from_image(&img);
            tex.set_filter(FilterMode::Linear);
            tex
        });
        // miniquad creates textures with `TextureWrap::Clamp`
        // (graphics.rs:405-418) and macroquad exposes no setter for it, but
        // every tiled surface asks for UVs past 1.0. Left clamped, those sample
        // the edge texel and smear it into stripes across the whole face.
        //
        // Mirrored rather than plain repeat: a tile's edge meets its own
        // reflection, so the join cannot be seen, while the pattern's visible
        // period doubles — the cheapest step against a 4 m tile reading as a
        // grid across a whole arena. The UVs are world coordinates, so the flip
        // lines fall on the same world grid the tiles are laid out on.
        //
        // SAFETY: this only sets the sampler parameters of textures created
        // right here, before anything has been drawn with them. It does not
        // touch the draw-call batching state `get_internal_gl` also hands out.
        unsafe {
            let gl = macroquad::window::get_internal_gl();
            for tex in &textures {
                gl.quad_context.texture_set_wrap(
                    tex.raw_miniquad_id(),
                    macroquad::miniquad::TextureWrap::Mirror,
                    macroquad::miniquad::TextureWrap::Mirror,
                );
            }
        }
        Textures(textures)
    }

    pub fn get(&self, surface: Surface) -> &Texture2D {
        &self.0[surface.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mean_rgb(img: &Image) -> [f32; 3] {
        let mut sum = [0.0f32; 3];
        let mut count = 0.0f32;
        for p in img.bytes.chunks_exact(4) {
            for c in 0..3 {
                sum[c] += p[c] as f32 / 255.0;
            }
            count += 1.0;
        }
        [sum[0] / count, sum[1] / count, sum[2] / count]
    }

    /// The table in `Surface::mean` is what keeps each arena the colour its
    /// palette says it is. If a texture is ever re-exported at a different
    /// exposure or hue, every level using it quietly shifts — and nothing else
    /// in the suite would notice, because every other check is about geometry.
    #[test]
    fn the_mean_table_matches_the_textures() {
        for surface in Surface::ALL {
            let img = Image::from_file_with_format(surface.png(), Some(ImageFormat::Png))
                .expect("a bundled surface texture is not a decodable PNG");
            let measured = mean_rgb(&img);
            let tabled = surface.mean();
            for c in 0..3 {
                assert!(
                    (measured[c] - tabled[c]).abs() < 0.005,
                    "{surface:?} channel {c}: the table says {}, the texture measures {}",
                    tabled[c],
                    measured[c],
                );
            }
        }
    }

    /// Every surface has to be a distinct texture, or a map that names two
    /// materials gets one of them twice.
    #[test]
    fn every_surface_has_its_own_texture() {
        for (i, a) in Surface::ALL.iter().enumerate() {
            assert_eq!(a.index(), i, "{a:?} is not at its own index");
            for b in &Surface::ALL[i + 1..] {
                assert_ne!(a.png(), b.png(), "{a:?} and {b:?} share a file");
            }
        }
    }

    fn luminance(c: [f32; 3]) -> f32 {
        0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
    }

    /// A block is painted with its palette colour divided by the texture's
    /// mean, but a vertex colour cannot exceed 1 — so a palette colour brighter
    /// than the texture under it cannot come out at the brightness the palette
    /// asks for. The failure is silent: the level just goes dim, one material
    /// at a time, and nothing in the suite would say which one.
    ///
    /// This pins the shortfall at 15%, which is around where a material is
    /// simply the wrong one for a colour and the pairing has to change.
    #[test]
    fn every_block_can_be_painted_at_its_palette_brightness() {
        let mut dim = Vec::new();
        for layout in crate::maps::campaign() {
            for block in &layout.boxes {
                let skin = block.skin;
                let mean = skin.surface.mean();
                let tint = skin.uniform();
                // What the average pixel of this block comes out as, against
                // what the palette asked for.
                let lit = luminance([mean[0] * tint.r, mean[1] * tint.g, mean[2] * tint.b]);
                let want = luminance([skin.color.r, skin.color.g, skin.color.b]);
                if lit < want * 0.85 {
                    dim.push(format!(
                        "  {}: {:?} on {:?} comes out at {:.0}%",
                        layout.name_en,
                        skin.color,
                        skin.surface,
                        lit / want * 100.0,
                    ));
                }
            }
        }
        assert!(
            dim.is_empty(),
            "{} blocks cannot reach their palette colour:\n{}",
            dim.len(),
            dim.join("\n"),
        );
    }
}
