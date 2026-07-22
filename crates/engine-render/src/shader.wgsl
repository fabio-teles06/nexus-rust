struct Camera {
    view_projection: mat4x4<f32>,
};

@group(0)
@binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0)
    position: vec3<f32>,
    @location(1)
    normal: vec3<f32>,
    @location(2)
    model_column_0: vec4<f32>,
    @location(3)
    model_column_1: vec4<f32>,
    @location(4)
    model_column_2: vec4<f32>,
    @location(5)
    model_column_3: vec4<f32>,
    @location(6)
    instance_color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position)
    clip_position: vec4<f32>,
    @location(0)
    world_normal: vec3<f32>,
    @location(1)
    color: vec4<f32>,
};

@vertex
fn vs_main(
    input: VertexInput,
) -> VertexOutput {
    let model = mat4x4<f32>(
        input.model_column_0,
        input.model_column_1,
        input.model_column_2,
        input.model_column_3,
    );

    let world_position = model * vec4<f32>(
        input.position,
        1.0,
    );

    let transformed_normal = model * vec4<f32>(
        input.normal,
        0.0,
    );

    var output: VertexOutput;

    output.clip_position = camera.view_projection
        * world_position;

    output.world_normal = normalize(
        transformed_normal.xyz,
    );

    output.color = input.instance_color;

    return output;
}

@fragment
fn fs_main(
    input: VertexOutput,
) -> @location(0) vec4<f32> {
    let light_direction = normalize(
        vec3<f32>(
            0.45,
            1.0,
            0.3,
        ),
    );

    let diffuse = max(
        dot(
            input.world_normal,
            light_direction,
        ),
        0.0,
    );

    let lighting = 0.28
        + diffuse * 0.72;

    return vec4<f32>(
        input.color.rgb * lighting,
        input.color.a,
    );
}