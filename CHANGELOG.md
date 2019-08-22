# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Depth testing support. It can be easily enabled using the new
  `GlyphBrushBuilder::depth_stencil_state` method. [#13]

### Changed
- `wgpu` dependency has been bumped to version `0.3`.

### Fixed
- Incorrect use of old cache on resize, causing validation errors and panics. [#9]

[#9]: https://github.com/hecrj/wgpu_glyph/pull/9
[#13]: https://github.com/hecrj/wgpu_glyph/pull/13


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


[Unreleased]: https://github.com/hecrj/wgpu_glyph/compare/0.3.1...HEAD
[0.3.1]: https://github.com/hecrj/wgpu_glyph/compare/0.3.0...0.3.1
[0.3.0]: https://github.com/hecrj/wgpu_glyph/compare/0.2.0...0.3.0
[0.2.0]: https://github.com/hecrj/wgpu_glyph/compare/0.1.1...0.2.0
[0.1.1]: https://github.com/hecrj/wgpu_glyph/compare/0.1.0...0.1.1
[0.1.0]: https://github.com/hecrj/wgpu_glyph/releases/tag/0.1.0
