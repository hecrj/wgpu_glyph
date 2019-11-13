//! A fast text renderer for [`wgpu`]. Powered by [`glyph_brush`].
//!
//! [`wgpu`]: https://github.com/gfx-rs/wgpu
//! [`glyph_brush`]: https://github.com/alexheretic/glyph-brush/tree/master/glyph-brush
#![deny(unused_results)]
mod builder;
mod pipeline;
mod region;

pub use region::Region;

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
pub struct GlyphBrush<'font, Depth, H = DefaultSectionHasher> {
    pipeline: Pipeline<Depth>,
    glyph_brush: glyph_brush::GlyphBrush<'font, Instance, H>,
}

impl<'font, Depth, H: BuildHasher> GlyphBrush<'font, Depth, H> {
    /// Queues a section/layout to be drawn by the next call of
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

    fn process_queued(
        &mut self,
        device: &mut wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let pipeline = &mut self.pipeline;

        let mut brush_action;

        loop {
            brush_action = self.glyph_brush.process_queued(
                |rect, tex_data| {
                    let offset = [rect.min.x as u16, rect.min.y as u16];
                    let size = [rect.width() as u16, rect.height() as u16];

                    pipeline
                        .update_cache(device, encoder, offset, size, tex_data);
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

                    pipeline.increase_cache_size(device, new_width, new_height);
                    self.glyph_brush.resize_texture(new_width, new_height);
                }
            }
        }

        match brush_action.unwrap() {
            BrushAction::Draw(verts) => {
                self.pipeline.upload(device, encoder, &verts);
            }
            BrushAction::ReDraw => {}
        };
    }

    /// Returns the available fonts.
    ///
    /// The `FontId` corresponds to the index of the font data.
    #[inline]
    pub fn fonts(&self) -> &[Font<'_>] {
        self.glyph_brush.fonts()
    }

    /// Adds an additional font to the one(s) initially added on build.
    ///
    /// Returns a new [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font_bytes<'a: 'font, B: Into<SharedBytes<'a>>>(
        &mut self,
        font_data: B,
    ) -> FontId {
        self.glyph_brush.add_font_bytes(font_data)
    }

    /// Adds an additional font to the one(s) initially added on build.
    ///
    /// Returns a new [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font<'a: 'font>(&mut self, font_data: Font<'a>) -> FontId {
        self.glyph_brush.add_font(font_data)
    }
}

impl<'font, H: BuildHasher> GlyphBrush<'font, (), H> {
    fn new(
        device: &mut wgpu::Device,
        filter_mode: wgpu::FilterMode,
        render_format: wgpu::TextureFormat,
        raw_builder: glyph_brush::GlyphBrushBuilder<'font, H>,
    ) -> Self {
        let glyph_brush = raw_builder.build();
        let (cache_width, cache_height) = glyph_brush.texture_dimensions();
        GlyphBrush {
            pipeline: Pipeline::<()>::new(
                device,
                filter_mode,
                render_format,
                cache_width,
                cache_height,
            ),
            glyph_brush,
        }
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
        self.draw_queued_with_transform(
            device,
            encoder,
            target,
            orthographic_projection(target_width, target_height),
        )
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
    #[inline]
    pub fn draw_queued_with_transform(
        &mut self,
        device: &mut wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        transform: [f32; 16],
    ) -> Result<(), String> {
        self.process_queued(device, encoder);
        self.pipeline.draw(device, encoder, target, transform, None);

        Ok(())
    }

    /// Draws all queued sections onto a render target, applying a position
    /// transform (e.g. a projection) and a scissoring region.
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
    pub fn draw_queued_with_transform_and_scissoring(
        &mut self,
        device: &mut wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        transform: [f32; 16],
        region: Region,
    ) -> Result<(), String> {
        self.process_queued(device, encoder);
        self.pipeline
            .draw(device, encoder, target, transform, Some(region));

        Ok(())
    }
}

impl<'font, H: BuildHasher>
    GlyphBrush<'font, wgpu::DepthStencilStateDescriptor, H>
{
    fn new(
        device: &mut wgpu::Device,
        filter_mode: wgpu::FilterMode,
        render_format: wgpu::TextureFormat,
        depth_stencil_state: wgpu::DepthStencilStateDescriptor,
        raw_builder: glyph_brush::GlyphBrushBuilder<'font, H>,
    ) -> Self {
        let glyph_brush = raw_builder.build();
        let (cache_width, cache_height) = glyph_brush.texture_dimensions();
        GlyphBrush {
            pipeline: Pipeline::<wgpu::DepthStencilStateDescriptor>::new(
                device,
                filter_mode,
                render_format,
                depth_stencil_state,
                cache_width,
                cache_height,
            ),
            glyph_brush,
        }
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
        depth_stencil_attachment: wgpu::RenderPassDepthStencilAttachmentDescriptor<
            &wgpu::TextureView,
        >,
        target_width: u32,
        target_height: u32,
    ) -> Result<(), String> {
        self.draw_queued_with_transform(
            device,
            encoder,
            target,
            depth_stencil_attachment,
            orthographic_projection(target_width, target_height),
        )
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
    #[inline]
    pub fn draw_queued_with_transform(
        &mut self,
        device: &mut wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_stencil_attachment: wgpu::RenderPassDepthStencilAttachmentDescriptor<
            &wgpu::TextureView,
        >,
        transform: [f32; 16],
    ) -> Result<(), String> {
        self.process_queued(device, encoder);
        self.pipeline.draw(
            device,
            encoder,
            target,
            depth_stencil_attachment,
            transform,
        );

        Ok(())
    }
}

// Helpers
fn orthographic_projection(width: u32, height: u32) -> [f32; 16] {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    [
        2.0 / width as f32, 0.0, 0.0, 0.0,
        0.0, 2.0 / height as f32, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        -1.0, -1.0, 0.0, 1.0,
    ]
}

impl<'font, D, H: BuildHasher> GlyphCruncher<'font>
    for GlyphBrush<'font, D, H>
{
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
