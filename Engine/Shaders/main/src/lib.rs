#![no_std]

use spirv_std::glam::{Mat4, Vec2, Vec3, Vec4};
use spirv_std::spirv;

#[repr(C)]
pub struct ModelViewProjection {
    pub model: Mat4,
    pub view: Mat4,
    pub proj: Mat4,
}

#[spirv(vertex)]
pub fn main_vs(
    // ── inputs ──────────────────────────────────────────────────
    #[spirv(location = 0)] in_position: Vec3,
    #[spirv(location = 1)] in_color: Vec3,
    #[spirv(location = 2)] in_tex_coord: Vec2,

    // ── uniform buffer at set 0, binding 0 ──────────────────────
    #[spirv(uniform, descriptor_set = 0, binding = 0)] mvp: &ModelViewProjection,

    // ── outputs ─────────────────────────────────────────────────
    #[spirv(location = 0)] frag_color: &mut Vec3,
    #[spirv(location = 1)] frag_tex_coord: &mut Vec2,
    #[spirv(position)] out_pos: &mut Vec4,
) {
    // Reconstruct gl_Position = proj * view * model * vec4(inPosition, 1.0)
    let world_pos = mvp.proj * mvp.view * mvp.model * in_position.extend(1.0);

    *out_pos = world_pos;
    *frag_color = in_color;
    *frag_tex_coord = in_tex_coord;
}

#[spirv(fragment)]
pub fn main_fs(
    // ── inputs (must match vert output locations exactly) ────────
    #[spirv(location = 0)] _frag_color: Vec3, // unused, matches vert output
    #[spirv(location = 1)] frag_tex_coord: Vec2,

    // ── combined image sampler at set 0, binding 1 ───────────────
    #[spirv(descriptor_set = 0, binding = 1)] tex_sampler: &spirv_std::image::SampledImage<
        spirv_std::Image!(2D, type=f32, sampled),
    >,

    // ── output ───────────────────────────────────────────────────
    #[spirv(location = 0)] out_color: &mut spirv_std::glam::Vec4,
) {
    *out_color = tex_sampler.sample(frag_tex_coord);
}
