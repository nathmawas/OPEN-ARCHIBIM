// The 3D view: flat colours lit by one sun and a sky.

struct Uniforms {
    view_proj: mat4x4<f32>,
    sun: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
) -> Out {
    var out: Out;
    out.position = u.view_proj * vec4<f32>(position, 1.0);
    let light = 0.55 + 0.3 * max(dot(normal, u.sun.xyz), 0.0) + 0.15 * max(normal.z, 0.0);
    out.color = color * light;
    return out;
}

@fragment
fn fs(in: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
