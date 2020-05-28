# wgpu_glyph

[![Integration status](https://github.com/hecrj/wgpu_glyph/workflows/Integration/badge.svg)](https://github.com/hecrj/wgpu_glyph/actions)
[![crates.io](https://img.shields.io/crates/v/wgpu_glyph.svg)](https://crates.io/crates/wgpu_glyph)
[![Documentation](https://docs.rs/wgpu_glyph/badge.svg)](https://docs.rs/wgpu_glyph)
[![License](https://img.shields.io/crates/l/wgpu_glyph.svg)](https://github.com/hecrj/wgpu_glyph/blob/master/LICENSE)

A fast text renderer for [wgpu](https://github.com/gfx-rs/wgpu), powered by
[glyph_brush](https://github.com/alexheretic/glyph-brush/tree/master/glyph-brush)

```rust
use wgpu_glyph::{ab_glyph, GlyphBrushBuilder, Section, Text};

let font = ab_glyph::FontArc::try_from_slice(include_bytes!("SomeFont.ttf"))
    .expect("Load font");

let mut glyph_brush = GlyphBrushBuilder::using_font(font)
    .build(&device, render_format);

let section = Section {
    screen_position: (10.0, 10.0),
    text: vec![Text::new("Hello wgpu_glyph")],
    ..Section::default()
};

glyph_brush.queue(section);

glyph_brush.draw_queued(
    &device,
    &mut encoder,
    &frame.view,
    frame.width,
    frame.height,
);

device.get_queue().submit(&[encoder.finish()]);
```

## Examples

Have a look at
  * `cargo run --example hello`
  * [Coffee](https://github.com/hecrj/coffee), which uses `wgpu_glyph` to
    provide font rendering on the [`wgpu` graphics backend].

[`wgpu` graphics backend]: https://github.com/hecrj/coffee/tree/master/src/graphics/backend_wgpu
