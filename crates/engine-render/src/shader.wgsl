struct Camera { view_projection: mat4x4<f32>, };
@group(0) @binding(0) var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) m0: vec4<f32>,
    @location(3) m1: vec4<f32>,
    @location(4) m2: vec4<f32>,
    @location(5) m3: vec4<f32>,
    @location(6) color: vec4<f32>,
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
};
@vertex fn vs_main(input: VertexInput) -> VertexOutput {
    let model = mat4x4<f32>(input.m0, input.m1, input.m2, input.m3);
    var out: VertexOutput;
    out.clip_position = camera.view_projection * model * vec4<f32>(input.position, 1.0);
    out.normal = normalize((model * vec4<f32>(input.normal, 0.0)).xyz);
    out.color = input.color;
    return out;
}
@fragment fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let light = normalize(vec3<f32>(0.4, 1.0, 0.3));
    let diffuse = max(dot(input.normal, light), 0.0);
    return vec4<f32>(input.color.rgb * (0.25 + diffuse * 0.75), input.color.a);
}
