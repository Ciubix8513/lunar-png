@group(1)@binding(0)
var texture: texture_2d<f32>;
@group(1)@binding(1)
var tex_sampler: sampler;

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0)
  uv: vec2<f32>,
}

@fragment
fn main(in: VertexOutput) -> @location(0) vec4<f32> {
    let col = textureSample(texture, tex_sampler, in.uv);

    return col;

    // let normalized_uv = (in.uv + vec2(1, 1)) / 2;
    // return vec4(in.uv, 0, 1.0);
}
