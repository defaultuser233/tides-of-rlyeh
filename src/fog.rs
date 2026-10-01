//! Distance fog for the world pass.
//!
//! The arenas are 52 m across and the geometry in them is drawn flat-shaded
//! with no aerial perspective at all, so a wall a hundred metres away arrives at
//! the eye as bright as the one you are standing next to and then stops against
//! the sky with a hard edge. Fading the far end into the sky's horizon colour is
//! the cheapest thing that stops that, and it is the one bit of "depth" that
//! costs the CPU nothing at all.
//!
//! The fog is a custom material, which is what a `Mesh` needs in order to be
//! shaded by anything other than macroquad's default 3D shader. Three facts
//! about that path were read out of the vendored source rather than assumed:
//!
//! - `load_material` starts from the built-in 3D shader's meta
//!   (`quad_gl.rs:627`), so a custom material gets `Projection`, `Model` and
//!   `_Time` in its uniform block for free, and image slot 0 stays bound to
//!   whatever `draw_mesh` was handed (`quad_gl.rs:744`). Nothing has to be
//!   re-declared except the fog's own uniforms.
//! - The depth state is *not* inherited from the default material, so it is
//!   copied here from the built-in 3D pipelines (`quad_gl.rs:444-449`).
//!   Getting that wrong draws the world in submission order.
//! - GLSL ES 100 always works: miniquad translates it for desktop GL, and says
//!   so in `graphics/gl.rs:841`. Writing a desktop `#version 330` shader instead
//!   would work here and break on the first driver that disagreed.

use macroquad::prelude::*;

const VERTEX: &str = r#"#version 100
    attribute vec3 position;
    attribute vec2 texcoord;
    attribute vec4 color0;
    attribute vec4 normal;

    varying lowp vec2 uv;
    varying lowp vec4 color;
    varying mediump float fog;

    uniform mat4 Model;
    uniform mat4 Projection;
    uniform vec4 FogRange;

    void main() {
        // `Model` is the view matrix, so this is the vertex in eye space and
        // its length is the distance to the eye in metres — the units the
        // range is written in, with no camera position to pass in.
        vec4 eye = Model * vec4(position, 1.0);
        gl_Position = Projection * eye;
        color = color0 / 255.0;
        uv = texcoord;
        fog = clamp((length(eye.xyz) - FogRange.x) / max(FogRange.y - FogRange.x, 0.001), 0.0, 1.0);
    }"#;

const FRAGMENT: &str = r#"#version 100
    varying lowp vec2 uv;
    varying lowp vec4 color;
    varying mediump float fog;

    uniform sampler2D Texture;
    uniform vec4 FogColour;

    void main() {
        vec4 texel = color * texture2D(Texture, uv);
        gl_FragColor = vec4(mix(texel.rgb, FogColour.rgb, fog * FogColour.a), texel.a);
    }"#;

/// How far away the fog starts, as a fraction of the arena's half-extent.
const START: f32 = 0.62;
/// Where it is fully opaque. Past the far wall, so the wall itself is hazed.
const END: f32 = 2.05;

pub struct Fog {
    material: Material,
}

impl Fog {
    /// Compile the fog material, or `None` if the driver would not take it.
    ///
    /// A shader that will not compile is not a reason to refuse to draw the
    /// world, so the caller falls back to the default material and the game
    /// looks exactly as it did before this module existed.
    pub fn load() -> Option<Self> {
        let params = MaterialParams {
            // Copied from the built-in 3D pipelines; a material that does not
            // ask for these draws the arena in whatever order it was submitted.
            pipeline_params: PipelineParams {
                depth_write: true,
                depth_test: Comparison::LessOrEqual,
                primitive_type: macroquad::miniquad::PrimitiveType::Triangles,
                ..Default::default()
            },
            uniforms: vec![
                UniformDesc::new("FogRange", UniformType::Float4),
                UniformDesc::new("FogColour", UniformType::Float4),
            ],
            textures: vec![],
        };
        let source = ShaderSource::Glsl {
            vertex: VERTEX,
            fragment: FRAGMENT,
        };
        match load_material(source, params) {
            Ok(material) => Some(Self { material }),
            Err(error) => {
                error!("fog shader would not compile: {error}");
                None
            }
        }
    }

    /// Begin a fogged pass: everything drawn until [`Fog::end`] fades into
    /// `colour` with distance.
    ///
    /// `half` is the arena's half-extent, so the range follows the map rather
    /// than being four numbers that have to be kept in step with it.
    pub fn begin(&self, colour: Color, half: f32) {
        self.material
            .set_uniform("FogRange", Vec4::new(half * START, half * END, 0.0, 0.0));
        // `a` is how far to the fog colour a fully fogged pixel gets. At 1.0 the
        // far wall would be indistinguishable from the sky, which loses the
        // silhouette the level is built out of.
        self.material
            .set_uniform("FogColour", Vec4::new(colour.r, colour.g, colour.b, 0.86));
        gl_use_material(&self.material);
    }

    pub fn end() {
        gl_use_default_material();
    }
}
