# wgpu_glyph

> [!WARNING]
> This crate has been superseded by [`glyphon`].
>
> [`glyphon`] has a simpler design that fits better with [`wgpu`]. Furthermore, it is built on top of [`cosmic-text`], which supports many more advanced text use cases.

[`glyphon`]: https://github.com/grovesNL/glyphon
[`wgpu`]: https://github.com/gfx-rs/wgpu
[`cosmic-text`]: https://github.com/pop-os/cosmic-text

[![Test Status](https://img.shields.io/github/actions/workflow/status/hecrj/wgpu_glyph/test.yml?branch=master&event=push&label=test)](https://github.com/hecrj/wgpu_glyph/actions)
[![crates.io](https://img.shields.io/crates/v/wgpu_glyph.svg)](https://crates.io/crates/wgpu_glyph)
[![Documentation](https://docs.rs/wgpu_glyph/badge.svg)](https://docs.rs/wgpu_glyph)
[![License](https://img.shields.io/crates/l/wgpu_glyph.svg)](https://github.com/hecrj/wgpu_glyph/blob/master/LICENSE)

A fast text renderer for [wgpu](https://github.com/gfx-rs/wgpu), powered by
[glyph_brush](https://github.com/alexheretic/glyph-brush/tree/master/glyph-brush).

## Examples

Have a look at
  * [The examples directory](examples).
  * [Iced](https://github.com/hecrj/iced), a cross-platform GUI library.
  * [Coffee](https://github.com/hecrj/coffee), a 2D game library.
