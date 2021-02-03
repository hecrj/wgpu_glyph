# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Changed
- Updated `wgpu` to `0.7`. [#50]
- Replaced `zerocopy` with `bytemuck`. [#51]

[#50]: https://github.com/hecrj/wgpu_glyph/pull/50
[#51]: https://github.com/hecrj/wgpu_glyph/pull/51


## [0.10.0] - 2020-08-27
### Changed
- Updated `wgpu` to `0.6`. [#44]
- Introduced `StagingBelt` for uploading data. [#46]

[#44]: https://github.com/hecrj/wgpu_glyph/pull/44
[#46]: https://github.com/hecrj/wgpu_glyph/pull/46


## [0.9.0] - 2020-04-29
### Added
- `orthographic_projection` helper to easily build a projection matrix. [#39]

### Changed
- Updated `glyph_brush` to `0.7`. [#43]

[#39]: https://github.com/hecrj/wgpu_glyph/pull/39
[#43]: https://github.com/hecrj/wgpu_glyph/pull/43


## [0.8.0] - 2020-04-13
### Changed
- `wgpu` dependency updated to `0.5`. [#33]
- The Y axis has been flipped to match the new NDC sytem in `wgpu`. [#33]

[#33]: https://github.com/hecrj/wgpu_glyph/pull/33


## [0.7.0] - 2020-03-02
### Changed
- `GlyphBrush::build` and `GlyphBrush::draw_queued*` methods take an immutable reference of a `wgpu::Device` now. [#29] [#30]
- `GlyphBrush::using_font_bytes` and `GlyphBrush::using_fonts_bytes` return an error instead of panicking when the provided font cannot be loaded. [#27]

[#27]: https://github.com/hecrj/wgpu_glyph/pull/27
[#29]: https://github.com/hecrj/wgpu_glyph/pull/29
[#30]: https://github.com/hecrj/wgpu_glyph/pull/30


## [0.6.0] - 2019-11-24
### Added
- `GlyphBrush::add_font_bytes` and `GlyphBrush::add_font`, which allow loading fonts after building a `GlyphBrush` [#25]
- `GlyphBrush::draw_queued_with_transform_and_scissoring`, which allows clipping text in the given `Region`. [#25]

[#25]: https://github.com/hecrj/wgpu_glyph/pull/25


## [0.5.0] - 2019-11-05
### Added
- `From<glyph_brush::GlyphBrushBuilder>` implementation for `wgpu_glyph::GlyphBrushBuilder`. [#19]

### Changed
- `glyph-brush` dependency updated to `0.6`. [#21]
- `wgpu` dependency updated to `0.4`. [#24]

[#19]: https://github.com/hecrj/wgpu_glyph/pull/19
[#21]: https://github.com/hecrj/wgpu_glyph/pull/21
[#24]: https://github.com/hecrj/wgpu_glyph/pull/24


## [0.4.0] - 2019-10-23
### Added
- Depth testing support. It can be easily enabled using the new
  `GlyphBrushBuilder::depth_stencil_state` method. [#13]

### Changed
- `wgpu` dependency has been bumped to version `0.3`. [#17]

### Fixed
- Incorrect use of old cache on resize, causing validation errors and panics. [#9]

[#9]: https://github.com/hecrj/wgpu_glyph/pull/9
[#13]: https://github.com/hecrj/wgpu_glyph/pull/13
[#17]: https://github.com/hecrj/wgpu_glyph/pull/17


## [0.3.1] - 2019-06-09
### Fixed
- Panic when drawing an empty `GlyphBrush`.


## [0.3.0] - 2019-05-03
### Added
- This changelog.

### Changed
- The transformation matrix in `draw_queued_with_transform` will be applied
  directly to vertices in absolute coordinates. This makes reusing vertices with
  different projections much easier. See [glyph_brush/pull/64].

### Removed
- Target dimensions in `draw_queued_with_transform`. The transform needs to take
  care of translating vertices coordinates into the normalized space.

[glyph_brush/pull/64]: https://github.com/alexheretic/glyph-brush/pull/64


## [0.2.0] - 2019-04-28
### Added
- Configurable render target format.


## [0.1.1] - 2019-04-27
### Fixed
- Instance buffer resize issue. Now, the instance buffer resizes when necessary.


## 0.1.0 - 2019-04-27
### Added
- First release! :tada:


[Unreleased]: https://github.com/hecrj/wgpu_glyph/compare/0.10.0...HEAD
[0.10.0]: https://github.com/hecrj/wgpu_glyph/compare/0.9.0...0.10.0
[0.9.0]: https://github.com/hecrj/wgpu_glyph/compare/0.8.0...0.9.0
[0.8.0]: https://github.com/hecrj/wgpu_glyph/compare/0.7.0...0.8.0
[0.7.0]: https://github.com/hecrj/wgpu_glyph/compare/0.6.0...0.7.0
[0.6.0]: https://github.com/hecrj/wgpu_glyph/compare/0.5.0...0.6.0
[0.5.0]: https://github.com/hecrj/wgpu_glyph/compare/0.4.0...0.5.0
[0.4.0]: https://github.com/hecrj/wgpu_glyph/compare/0.3.1...0.4.0
[0.3.1]: https://github.com/hecrj/wgpu_glyph/compare/0.3.0...0.3.1
[0.3.0]: https://github.com/hecrj/wgpu_glyph/compare/0.2.0...0.3.0
[0.2.0]: https://github.com/hecrj/wgpu_glyph/compare/0.1.1...0.2.0
[0.1.1]: https://github.com/hecrj/wgpu_glyph/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/hecrj/wgpu_glyph/releases/tag/0.1.0
