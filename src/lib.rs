//! A fast text renderer for [`wgpu`]. Powered by [`glyph_brush`].
//!
//! [`wgpu`]: https://github.com/gfx-rs/wgpu
//! [`glyph_brush`]: https://github.com/alexheretic/glyph-brush/tree/master/glyph-brush
mod builder;
mod pipeline;

use pipeline::{Instance, Pipeline};

pub use builder::GlyphBrushBuilder;
pub use glyph_brush::{
    rusttype::{self, Font, Point, PositionedGlyph, Rect, Scale, SharedBytes},
    BuiltInLineBreaker, FontId, FontMap, GlyphCruncher, GlyphPositioner,
    HorizontalAlign, Layout, LineBreak, LineBreaker, OwnedSectionText,
    OwnedVariedSection, PositionedGlyphIter, Section, SectionGeometry,
    SectionText, VariedSection, VerticalAlign,
};

use core::hash::BuildHasher;
use std::borrow::Cow;

use glyph_brush::{BrushAction, BrushError, Color, DefaultSectionHasher};
use log::{log_enabled, warn};

/// Object allowing glyph drawing, containing cache state. Manages glyph positioning cacheing,
/// glyph draw caching & efficient GPU texture cache updating and re-sizing on demand.
///
/// Build using a [`GlyphBrushBuilder`](struct.GlyphBrushBuilder.html).
pub struct GlyphBrush<'font, H = DefaultSectionHasher> {
    pipeline: Pipeline,
    glyph_brush: glyph_brush::GlyphBrush<'font, Instance, H>,
}

impl<'font, H: BuildHasher> GlyphBrush<'font, H> {
    fn new(
        device: &mut wgpu::Device,
        filter_mode: wgpu::FilterMode,
        render_format: wgpu::TextureFormat,
        raw_builder: glyph_brush::GlyphBrushBuilder<'font, H>,
    ) -> Self {
        let (cache_width, cache_height) = raw_builder.initial_cache_size;

        let pipeline = Pipeline::new(
            device,
            filter_mode,
            render_format,
            cache_width,
            cache_height,
        );

        GlyphBrush {
            pipeline: pipeline,
            glyph_brush: raw_builder.build(),
        }
    }

    // Queues a section/layout to be drawn by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be
    /// called multiple times to queue multiple sections for drawing.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn queue<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        self.glyph_brush.queue(section)
    }

    /// Queues a section/layout to be drawn by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be
    /// called multiple times to queue multiple sections for drawing.
    ///
    /// Used to provide custom `GlyphPositioner` logic, if using built-in
    /// [`Layout`](enum.Layout.html) simply use
    /// [`queue`](struct.GlyphBrush.html#method.queue)
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn queue_custom_layout<'a, S, G>(
        &mut self,
        section: S,
        custom_layout: &G,
    ) where
        G: GlyphPositioner,
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        self.glyph_brush.queue_custom_layout(section, custom_layout)
    }

    /// Queues pre-positioned glyphs to be processed by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be
    /// called multiple times.
    #[inline]
    pub fn queue_pre_positioned(
        &mut self,
        glyphs: Vec<(PositionedGlyph<'font>, Color, FontId)>,
        bounds: Rect<f32>,
        z: f32,
    ) {
        self.glyph_brush.queue_pre_positioned(glyphs, bounds, z)
    }

    /// Retains the section in the cache as if it had been used in the last
    /// draw-frame.
    ///
    /// Should not be necessary unless using multiple draws per frame with
    /// distinct transforms, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn keep_cached_custom_layout<'a, S, G>(
        &mut self,
        section: S,
        custom_layout: &G,
    ) where
        S: Into<Cow<'a, VariedSection<'a>>>,
        G: GlyphPositioner,
    {
        self.glyph_brush
            .keep_cached_custom_layout(section, custom_layout)
    }

    /// Retains the section in the cache as if it had been used in the last
    /// draw-frame.
    ///
    /// Should not be necessary unless using multiple draws per frame with
    /// distinct transforms, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn keep_cached<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        self.glyph_brush.keep_cached(section)
    }

    /// Draws all queued sections onto a render target.
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// It __does not__ submit the encoder command buffer to the device queue.
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    ///
    /// # Panics
    /// Panics if the provided `target` has a texture format that does not match
    /// the `render_format` provided on creation of the `GlyphBrush`.
    #[inline]
    pub fn draw_queued(
        &mut self,
        device: &mut wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        target_width: u32,
        target_height: u32,
    ) -> Result<(), String> {
        #[cfg_attr(rustfmt, rustfmt_skip)]
        let projection =
            [
                2.0 / target_width as f32, 0.0, 0.0, 0.0,
                0.0, 2.0 / target_height as f32, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                -1.0, -1.0, 0.0, 1.0,
            ];

        self.draw_queued_with_transform(projection, device, encoder, target)
    }

    /// Draws all queued sections onto a render target, applying a position
    /// transform (e.g. a projection).
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// It __does not__ submit the encoder command buffer to the device queue.
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    ///
    /// # Panics
    /// Panics if the provided `target` has a texture format that does not match
    /// the `render_format` provided on creation of the `GlyphBrush`.
    pub fn draw_queued_with_transform(
        &mut self,
        transform: [f32; 16],
        device: &mut wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) -> Result<(), String> {
        let mut cache = self.pipeline.cache();

        let mut brush_action;

        loop {
            brush_action = self.glyph_brush.process_queued(
                |rect, tex_data| {
                    let offset = [rect.min.x as u16, rect.min.y as u16];
                    let size = [rect.width() as u16, rect.height() as u16];

                    cache.update(device, encoder, offset, size, tex_data);
                },
                Instance::from,
            );

            match brush_action {
                Ok(_) => break,
                Err(BrushError::TextureTooSmall { suggested }) => {
                    // TODO: Obtain max texture dimensions using `wgpu`
                    // This is currently not possible I think. Ask!
                    let max_image_dimension = 2048;

                    let (new_width, new_height) = if (suggested.0
                        > max_image_dimension
                        || suggested.1 > max_image_dimension)
                        && (self.glyph_brush.texture_dimensions().0
                            < max_image_dimension
                            || self.glyph_brush.texture_dimensions().1
                                < max_image_dimension)
                    {
                        (max_image_dimension, max_image_dimension)
                    } else {
                        suggested
                    };

                    if log_enabled!(log::Level::Warn) {
                        warn!(
                            "Increasing glyph texture size {old:?} -> {new:?}. \
                             Consider building with `.initial_cache_size({new:?})` to avoid \
                             resizing",
                            old = self.glyph_brush.texture_dimensions(),
                            new = (new_width, new_height),
                        );
                    }

                    self.pipeline
                        .increase_cache_size(device, new_width, new_height);
                    self.glyph_brush.resize_texture(new_width, new_height);

                    cache = self.pipeline.cache();
                }
            }
        }

        match brush_action.unwrap() {
            BrushAction::Draw(verts) => {
                self.pipeline.upload(device, encoder, &verts);
                self.pipeline.draw(device, encoder, target, transform);
            }
            BrushAction::ReDraw => {
                self.pipeline.draw(device, encoder, target, transform);
            }
        };

        Ok(())
    }

    /// Returns the available fonts.
    ///
    /// The `FontId` corresponds to the index of the font data.
    #[inline]
    pub fn fonts(&self) -> &[Font<'_>] {
        self.glyph_brush.fonts()
    }
}

impl<'font, H: BuildHasher> GlyphCruncher<'font> for GlyphBrush<'font, H> {
    #[inline]
    fn pixel_bounds_custom_layout<'a, S, L>(
        &mut self,
        section: S,
        custom_layout: &L,
    ) -> Option<Rect<i32>>
    where
        L: GlyphPositioner + std::hash::Hash,
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        self.glyph_brush
            .pixel_bounds_custom_layout(section, custom_layout)
    }

    #[inline]
    fn glyphs_custom_layout<'a, 'b, S, L>(
        &'b mut self,
        section: S,
        custom_layout: &L,
    ) -> PositionedGlyphIter<'b, 'font>
    where
        L: GlyphPositioner + std::hash::Hash,
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        self.glyph_brush
            .glyphs_custom_layout(section, custom_layout)
    }

    #[inline]
    fn fonts(&self) -> &[Font<'font>] {
        self.glyph_brush.fonts()
    }
}

impl<H> std::fmt::Debug for GlyphBrush<'_, H> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GlyphBrush")
    }
}
