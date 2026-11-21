// Enhanced Visuals' box blur, vertical half, then its saturation pass
// (`color_convolve.fsh`): `Chroma * Saturation + Luma`, with the original's own grey weights.
// Reads what `horizontal` wrote, so the blur is separable and the radii are the original's.

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
// `color_convolve.fsh`'s `const vec3 Gray = vec3(0.3, 0.59, 0.11)`.
const GRAY: vec3<f32> = vec3<f32>(0.3, 0.59, 0.11);

fn post(input: PostInput) -> vec4<f32> {
    let radius = max(frame.mods[0].x, 0.0);
    let saturation = frame.mods[0].y;

    var colour = input.color.rgb;
    if (radius >= 0.5) {
        let step = vec2<f32>(0.0, radius) * 0.3665 / max(frame.resolution, vec2<f32>(1.0));
        var total = input.color.rgb;
        for (var i = 1; i < 7; i = i + 1) {
            let offset = step * f32(i);
            total = total + (textureSampleLevel(blurred, linear_sampler, input.uv + offset, 0.0).rgb
                + textureSampleLevel(blurred, linear_sampler, input.uv - offset, 0.0).rgb) * WEIGHT[i];
        }
        colour = total / TOTAL;
    }

    // The original's desaturation runs as its own pass, over whatever the blur left.
    let luma = dot(colour, GRAY);
    colour = (colour - vec3<f32>(luma)) * saturation + vec3<f32>(luma);

    return vec4<f32>(colour, 1.0);
}
