mod builder;
mod pipeline;

use pipeline::{Instance, Pipeline};

pub use builder::GlyphBrushBuilder;
pub use glyph_brush::{BrushAction, BrushError, Section, VariedSection};

use core::hash::BuildHasher;
use std::borrow::Cow;

use glyph_brush::DefaultSectionHasher;
use log::{log_enabled, warn};

/// Object allowing glyph drawing, containing cache state. Manages glyph positioning cacheing,
/// glyph draw caching & efficient GPU texture cache updating and re-sizing on demand.
///
/// Build using a [`GlyphBrushBuilder`](struct.GlyphBrushBuilder.html).
///
/// # Caching behaviour
///
/// Calls to [`GlyphBrush::queue`](#method.queue),
/// [`GlyphBrush::pixel_bounds`](#method.pixel_bounds), [`GlyphBrush::glyphs`](#method.glyphs)
/// calculate the positioned glyphs for a section.
/// This is cached so future calls to any of the methods for the same section are much
/// cheaper. In the case of [`GlyphBrush::queue`](#method.queue) the calculations will also be
/// used for actual drawing.
///
/// The cache for a section will be **cleared** after a
/// [`GlyphBrush::draw_queued`](#method.draw_queued) call when that section has not been used since
/// the previous draw call.
pub struct GlyphBrush<'font, H = DefaultSectionHasher> {
    pipeline: Pipeline,
    glyph_brush: glyph_brush::GlyphBrush<'font, Instance, H>,
}

impl<'font, H: BuildHasher> GlyphBrush<'font, H> {
    fn new(
        device: &mut wgpu::Device,
        filter_method: wgpu::FilterMode,
        raw_builder: glyph_brush::GlyphBrushBuilder<'font, H>,
    ) -> Self {
        let (cache_width, cache_height) = raw_builder.initial_cache_size;

        let pipeline =
            Pipeline::new(device, filter_method, cache_width, cache_height);

        GlyphBrush {
            pipeline: pipeline,
            glyph_brush: raw_builder.build(),
        }
    }

    /// Queues a section/layout to be drawn by the next call of
    /// [`draw_queued`](struct.GlyphBrush.html#method.draw_queued). Can be called multiple times
    /// to queue multiple sections for drawing.
    ///
    /// Benefits from caching, see [caching behaviour](#caching-behaviour).
    #[inline]
    pub fn queue<'a, S>(&mut self, section: S)
    where
        S: Into<Cow<'a, VariedSection<'a>>>,
    {
        self.glyph_brush.queue(section)
    }

    /// Draws all queued sections onto a render target.
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    ///
    /// # Raw usage
    /// Can also be used with gfx raw render & depth views if necessary. The `Format` must also
    /// be provided. [See example.](struct.GlyphBrush.html#raw-usage-1)
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
            Pipeline::IDENTITY_MATRIX,
            device,
            encoder,
            target,
            target_width,
            target_height,
        )
    }

    /// Draws all queued sections onto a render target, applying a position transform (e.g.
    /// a projection).
    /// See [`queue`](struct.GlyphBrush.html#method.queue).
    ///
    /// Trims the cache, see [caching behaviour](#caching-behaviour).
    ///
    pub fn draw_queued_with_transform(
        &mut self,
        transform: [f32; 16],
        device: &mut wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        target_width: u32,
        target_height: u32,
    ) -> Result<(), String> {
        let cache = self.pipeline.cache();

        let mut brush_action;

        loop {
            brush_action = self.glyph_brush.process_queued(
                (target_width, target_height),
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
                }
            }
        }

        match brush_action.unwrap() {
            BrushAction::Draw(verts) => {
                self.pipeline.draw(device, encoder, transform, &verts);
            }
            BrushAction::ReDraw => {
                self.pipeline.redraw(encoder);
            }
        };

        Ok(())
    }
}
