// Enhanced Visuals' box blur, horizontal half, and the target the vertical half reads.
//
// The mod fills `frame.mods[0]` every frame (its `shader_params` hook):
//   x: the blur's radius in pixels of the world's picture, exactly the number the original
//      hands its own post chain (`VisualTypeBlurClient`: `max(1, floor(intensity))`), with a
//      chain per visual so the radii add up. 0 means nothing is blurring.
//   y: the `Saturation` the original's `color_convolve` pass gets, 1.0 being untouched.
//   z, w: unused, 0.
// With no mod, or with x at 0, the picture passes through untouched, so this pack can stay
// picked as a home shader.
//
// The original runs three rounds of a separable box blur of that radius. Three box rounds per
// axis are a Gaussian of standard deviation about one radius, so each half here samples a
// 13-tap Gaussian with sigma = radius: the same spread for two passes instead of six.
// Colours are linear light, 1.0 at paper white; the result's alpha is ignored.

// exp(-(i * 0.3665)^2 / 2) for i = 0..6: the weights of a Gaussian sampled 0.3665 sigma apart,
// normalised at the end.
const WEIGHT: array<f32, 7> = array<f32, 7>(
    1.0,
    0.934911,
    0.764618,
    0.546074,
    0.341392,
    0.186413,
    0.088922,
);
const TOTAL: f32 = 6.724658;

fn post(input: PostInput) -> vec4<f32> {
    let radius = max(frame.mods[0].x, 0.0);
    if (radius < 0.5) {
        return input.color;
    }
    // One sigma per radius, in texels of the world's picture.
    let step = vec2<f32>(radius, 0.0) * 0.3665 / max(frame.resolution, vec2<f32>(1.0));
    var total = input.color.rgb;
    for (var i = 1; i < 7; i = i + 1) {
        let offset = step * f32(i);
        total = total + (textureSampleLevel(previous, linear_sampler, input.uv + offset, 0.0).rgb
            + textureSampleLevel(previous, linear_sampler, input.uv - offset, 0.0).rgb) * WEIGHT[i];
    }
    return vec4<f32>(total / TOTAL, 1.0);
}
