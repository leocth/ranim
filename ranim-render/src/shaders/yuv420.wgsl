[[group(0), binding(0)]] var input_texture : texture_2d<f32>;
[[group(0), binding(1)]] var output_texture : texture_storage_2d<rgba8unorm, write>;

fn calculate_y(rgba: vec4<f32>) -> f32 {
    return dot(rgba, vec4<f32>(0.2578125, 0.50390625, 0.09765625, 0.0)) + (16.0 / 255.0);
}
fn calculate_u(rgba: vec4<f32>) -> f32 {
    return dot(rgba, vec4<f32>(-0.1484375, -0.2890625, 0.4375, 0.0)) + (128.0 / 255.0);
}
fn calculate_v(rgba: vec4<f32>) -> f32 {
    return dot(rgba, vec4<f32>(0.4375, -0.3671875, -0.0703125, 0.0)) + (128.0 / 255.0); 
}

[[stage(compute), workgroup_size(16, 16)]]
fn yuv_main(
  [[builtin(global_invocation_id)]] global_id: vec3<u32>,
) {
    let dimensions = textureDimensions(input_texture);
    let coords = vec2<i32>(global_id.xy);
    // if(coords.x >= dimensions.x || coords.y >= dimensions.y) {
    //     return;
    // }

    // let s = coords.xy / 2;
    // let c00 = coords.xy * 2;
    // let c01 = c00 + vec2<i32>(0, 1);
    // let c10 = c00 + vec2<i32>(1, 0);
    // let c11 = c00 + vec2<i32>(1, 1);

    // let p00 = textureLoad(input_texture, c00, 0);
    // let p01 = textureLoad(input_texture, c01, 0);
    // let p10 = textureLoad(input_texture, c10, 0);
    // let p11 = textureLoad(input_texture, c11, 0);
    // let pavg = (p00 + p01 + p10 + p11) / 4.0;

    // let pix = textureLoad(input_texture, coords.xy, 0);

    // let y = dot(pix.xyz, vec3<f32>(0.2578125, 0.50390625, 0.09765625)) + 0.114;
    // let u = dot(pavg.xyz, vec3<f32>(-0.1484375, -0.2890625, 0.4375)) + 0.5;
    // let v = dot(pavg.xyz, vec3<f32>(0.4375, -0.3671875, -0.0703125)) + 0.5;

    textureStore(output_texture, coords.xy, vec4<f32>(1.0, 1.0, 1.0, 1.0));
}