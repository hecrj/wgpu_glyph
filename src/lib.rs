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
pub use glyph_brush::ab_glyph;
pub use glyph_brush::{
    BuiltInLineBreaker, FontId, GlyphCruncher, GlyphPositioner,
    HorizontalAlign, Layout, LineBreak, LineBreaker, OwnedSection, OwnedText,
    SectionGeometry, SectionGlyph, SectionGlyphIter, SectionText,
    VerticalAlign,
};

use ab_glyph::{Font, Rect};
use core::hash::{BuildHasher, Hash};
use std::borrow::Cow;

use glyph_brush::{BrushAction, BrushError, DefaultSectionHasher};
use log::{log_enabled, warn};

#[derive(Debug, Clone, Copy)]
pub struct Extra {
    pub extra: glyph_brush::Extra,
    pub outline_color: glyph_brush::Color,
}

impl Hash for Extra {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        use ordered_float::OrderedFloat;
        self.extra.hash(state);
        [
            OrderedFloat::from(self.outline_color[0]),
            OrderedFloat::from(self.outline_color[1]),
            OrderedFloat::from(self.outline_color[2]),
            OrderedFloat::from(self.outline_color[3]),
        ]
        .hash(state)
    }
}
impl PartialEq for Extra {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.extra == other.extra && self.outline_color == other.outline_color
    }
}

impl Default for Extra {
    #[inline]
    fn default() -> Self {
        Self {
            extra: Default::default(),
            outline_color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}
pub type Section<'a> = glyph_brush::Section<'a, Extra>;
pub type Text<'a> = glyph_brush::Text<'a, Extra>;

pub trait TextExt {
    fn with_color<C: Into<glyph_brush::Color>>(self, color: C) -> Self;
    fn with_outline_color<C: Into<glyph_brush::Color>>(self, color: C) -> Self;
    fn with_z<Z: Into<f32>>(self, z: Z) -> Self;
}

impl<'a> TextExt for Text<'a> {
    #[inline]
    fn with_color<C: Into<glyph_brush::Color>>(mut self, color: C) -> Self {
        self.extra.extra.color = color.into();
        self
    }
    #[inline]
    fn with_outline_color<C: Into<glyph_brush::Color>>(
        mut self,
        color: C,
    ) -> Self {
        self.extra.outline_color = color.into();
        self
    }
    #[inline]
    fn with_z<Z: Into<f32>>(mut self, z: Z) -> Self {
        self.extra.extra.z = z.into();
        self
    }
}

impl std::ops::Deref for Extra {
    type Target = glyph_brush::Extra;
    fn deref(&self) -> &Self::Target {
        &self.extra
    }
}
impl std::ops::DerefMut for Extra {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.extra
    }
}

/// Object allowing glyph drawing, containing cache state. Manages glyph positioning cacheing,
/// glyph draw caching & efficient GPU texture cache updating and re-sizing on demand.
///
/// Build using a [`GlyphBrushBuilder`](struct.GlyphBrushBuilder.html).
pub struct GlyphBrush<Depth, F = ab_glyph::FontArc, H = DefaultSectionHasher> {
    pipeline: Pipeline<Depth>,
    glyph_brush: glyph_brush::GlyphBrush<Instance, Extra, F, H>,
}

impl<Depth, F: Font, H: BuildHasher> GlyphBrush<Depth, F, H> {
    /// Queues a section/layout to be drawn by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be
    /// called multiple times to queue multiple sections for drawing.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn queue<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, Section<'a>>>,
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
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.glyph_brush.queue_custom_layout(section, custom_layout)
    }

    /// Queues pre-positioned glyphs to be processed by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be
    /// called multiple times.
    #[inline]
    pub fn queue_pre_positioned(
        &mut self,
        glyphs: Vec<SectionGlyph>,
        extra: Vec<Extra>,
        bounds: Rect,
    ) {
        self.glyph_brush.queue_pre_positioned(glyphs, extra, bounds)
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
        S: Into<Cow<'a, Section<'a>>>,
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
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.glyph_brush.keep_cached(section)
    }

    /// Returns the available fonts.
    ///
    /// The `FontId` corresponds to the index of the font data.
    #[inline]
    pub fn fonts(&self) -> &[F] {
        self.glyph_brush.fonts()
    }

    /// Adds an additional font to the one(s) initially added on build.
    ///
    /// Returns a new [`FontId`](struct.FontId.html) to reference this font.
    pub fn add_font(&mut self, font: F) -> FontId {
        self.glyph_brush.add_font(font)
    }
}

impl<D, F, H> GlyphBrush<D, F, H>
where
    F: Font + Sync,
    H: BuildHasher,
{
    fn process_queued(
        &mut self,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let pipeline = &mut self.pipeline;

        let mut brush_action;

        loop {
            brush_action = self.glyph_brush.process_queued(
                |rect, tex_data| {
                    let offset = [rect.min[0] as u16, rect.min[1] as u16];
                    let size = [rect.width() as u16, rect.height() as u16];

                    pipeline.update_cache(
                        device,
                        staging_belt,
                        encoder,
                        offset,
                        size,
                        tex_data,
                    );
                },
                Instance::from_vertex,
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
                self.pipeline.upload(device, staging_belt, encoder, &verts);
            }
            BrushAction::ReDraw => {}
        };
    }
}

impl<F: Font + Sync, H: BuildHasher> GlyphBrush<(), F, H> {
    fn new(
        device: &wgpu::Device,
        filter_mode: wgpu::FilterMode,
        multisample: wgpu::MultisampleState,
        render_format: wgpu::TextureFormat,
        raw_builder: glyph_brush::GlyphBrushBuilder<F, H>,
    ) -> Self {
        let glyph_brush = raw_builder.build();
        let (cache_width, cache_height) = glyph_brush.texture_dimensions();
        GlyphBrush {
            pipeline: Pipeline::<()>::new(
                device,
                filter_mode,
                multisample,
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
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        target_width: u32,
        target_height: u32,
    ) -> Result<(), String> {
        self.draw_queued_with_transform(
            device,
            staging_belt,
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
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        transform: [f32; 16],
    ) -> Result<(), String> {
        self.process_queued(device, staging_belt, encoder);
        self.pipeline.draw(
            device,
            staging_belt,
            encoder,
            target,
            transform,
            None,
        );

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
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        transform: [f32; 16],
        region: Region,
    ) -> Result<(), String> {
        self.process_queued(device, staging_belt, encoder);
        self.pipeline.draw(
            device,
            staging_belt,
            encoder,
            target,
            transform,
            Some(region),
        );

        Ok(())
    }
}

impl<F: Font + Sync, H: BuildHasher> GlyphBrush<wgpu::DepthStencilState, F, H> {
    fn new(
        device: &wgpu::Device,
        filter_mode: wgpu::FilterMode,
        multisample: wgpu::MultisampleState,
        render_format: wgpu::TextureFormat,
        depth_stencil_state: wgpu::DepthStencilState,
        raw_builder: glyph_brush::GlyphBrushBuilder<F, H>,
    ) -> Self {
        let glyph_brush = raw_builder.build();
        let (cache_width, cache_height) = glyph_brush.texture_dimensions();
        GlyphBrush {
            pipeline: Pipeline::<wgpu::DepthStencilState>::new(
                device,
                filter_mode,
                multisample,
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
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_stencil_attachment: wgpu::RenderPassDepthStencilAttachment,
        target_width: u32,
        target_height: u32,
    ) -> Result<(), String> {
        self.draw_queued_with_transform(
            device,
            staging_belt,
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
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_stencil_attachment: wgpu::RenderPassDepthStencilAttachment,
        transform: [f32; 16],
    ) -> Result<(), String> {
        self.process_queued(device, staging_belt, encoder);
        self.pipeline.draw(
            device,
            staging_belt,
            encoder,
            target,
            depth_stencil_attachment,
            transform,
            None,
        );

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
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_stencil_attachment: wgpu::RenderPassDepthStencilAttachment,
        transform: [f32; 16],
        region: Region,
    ) -> Result<(), String> {
        self.process_queued(device, staging_belt, encoder);

        self.pipeline.draw(
            device,
            staging_belt,
            encoder,
            target,
            depth_stencil_attachment,
            transform,
            Some(region),
        );

        Ok(())
    }
}

/// Helper function to generate a generate a transform matrix.
pub fn orthographic_projection(width: u32, height: u32) -> [f32; 16] {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    [
        2.0 / width as f32, 0.0, 0.0, 0.0,
        0.0, -2.0 / height as f32, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        -1.0, 1.0, 0.0, 1.0,
    ]
}

impl<D, F: Font, H: BuildHasher> GlyphCruncher<F, Extra>
    for GlyphBrush<D, F, H>
{
    #[inline]
    fn glyphs_custom_layout<'a, 'b, S, L>(
        &'b mut self,
        section: S,
        custom_layout: &L,
    ) -> SectionGlyphIter<'b>
    where
        L: GlyphPositioner + std::hash::Hash,
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.glyph_brush
            .glyphs_custom_layout(section, custom_layout)
    }

    #[inline]
    fn fonts(&self) -> &[F] {
        self.glyph_brush.fonts()
    }

    #[inline]
    fn glyph_bounds_custom_layout<'a, S, L>(
        &mut self,
        section: S,
        custom_layout: &L,
    ) -> Option<Rect>
    where
        L: GlyphPositioner + std::hash::Hash,
        S: Into<Cow<'a, Section<'a>>>,
    {
        self.glyph_brush
            .glyph_bounds_custom_layout(section, custom_layout)
    }
}

impl<F, H> std::fmt::Debug for GlyphBrush<F, H> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GlyphBrush")
    }
}
