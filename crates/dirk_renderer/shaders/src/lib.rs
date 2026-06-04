#![no_std]
// shaders often have a lot of different inputs, we can't help it
#![allow(clippy::too_many_arguments)]

use spirv_std::{
    glam::{Vec2, Vec3, Vec4},
    image::{Image2d, SampledImage},
    spirv,
};

use dirk_shaders::types::{ProxyUbo, SceneUbo};

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] scene: &SceneUbo,
    #[spirv(uniform, descriptor_set = 1, binding = 0)] proxy: &ProxyUbo,
    #[spirv(location = 0)] in_position: Vec3,
    #[spirv(location = 1)] _in_normal: Vec3,
    #[spirv(location = 2)] in_tex_coord: Vec2,
    #[spirv(position)] out_position: &mut Vec4,
    #[spirv(location = 0)] frag_tex_coord: &mut Vec2,
) {
    *out_position = scene.proj * scene.view * proxy.model * in_position.extend(1.0);
    *frag_tex_coord = in_tex_coord;
}

#[spirv(fragment)]
pub fn main_fs(
    #[spirv(descriptor_set = 2, binding = 0)] tex_sampler: &SampledImage<Image2d>,
    #[spirv(location = 0)] frag_tex_coord: Vec2,
    #[spirv(location = 0)] out_color: &mut Vec4,
) {
    *out_color = tex_sampler.sample(frag_tex_coord);
}
