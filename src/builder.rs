use core::hash::BuildHasher;

use glyph_brush::delegate_glyph_brush_builder_fns;
use glyph_brush::{rusttype, DefaultSectionHasher};
use rusttype::{Font, SharedBytes};

use super::GlyphBrush;

/// Builder for a [`GlyphBrush`](struct.GlyphBrush.html).
pub struct GlyphBrushBuilder<'a, D, H = DefaultSectionHasher> {
    inner: glyph_brush::GlyphBrushBuilder<'a, H>,
    texture_filter_method: wgpu::FilterMode,
    depth: D,
}

impl<'a, H> From<glyph_brush::GlyphBrushBuilder<'a, H>> for GlyphBrushBuilder<'a, (), H> {
    fn from(inner: glyph_brush::GlyphBrushBuilder<'a, H>) -> Self {
        GlyphBrushBuilder {
            inner,
            texture_filter_method: wgpu::FilterMode::Linear,
            depth: (),
        }
    }
}

impl<'a> GlyphBrushBuilder<'a, ()> {
    /// Specifies the default font data used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    #[inline]
    pub fn using_font_bytes<B: Into<SharedBytes<'a>>>(font_0_data: B) -> Self {
        Self::using_font(Font::from_bytes(font_0_data).unwrap())
    }

    #[inline]
    pub fn using_fonts_bytes<B, V>(font_data: V) -> Self
    where
        B: Into<SharedBytes<'a>>,
        V: Into<Vec<B>>,
    {
        Self::using_fonts(
            font_data
                .into()
                .into_iter()
                .map(|data| Font::from_bytes(data).unwrap())
                .collect::<Vec<_>>(),
        )
    }

    /// Specifies the default font used to render glyphs.
    /// Referenced with `FontId(0)`, which is default.
    #[inline]
    pub fn using_font(font_0: Font<'a>) -> Self {
        Self::using_fonts(vec![font_0])
    }

    pub fn using_fonts<V: Into<Vec<Font<'a>>>>(fonts: V) -> Self {
        GlyphBrushBuilder {
            inner: glyph_brush::GlyphBrushBuilder::using_fonts(fonts),
            texture_filter_method: wgpu::FilterMode::Linear,
            depth: (),
        }
    }
}

impl<'a, D, H: BuildHasher> GlyphBrushBuilder<'a, D, H> {
    delegate_glyph_brush_builder_fns!(inner);

    /// Sets the texture filtering method.
    pub fn texture_filter_method(
        mut self,
        filter_method: wgpu::FilterMode,
    ) -> Self {
        self.texture_filter_method = filter_method;
        self
    }

    /// Sets the section hasher. `GlyphBrush` cannot handle absolute section
    /// hash collisions so use a good hash algorithm.
    ///
    /// This hasher is used to distinguish sections, rather than for hashmap
    /// internal use.
    ///
    /// Defaults to [seahash](https://docs.rs/seahash).
    pub fn section_hasher<T: BuildHasher>(
        self,
        section_hasher: T,
    ) -> GlyphBrushBuilder<'a, D, T> {
        GlyphBrushBuilder {
            inner: self.inner.section_hasher(section_hasher),
            texture_filter_method: self.texture_filter_method,
            depth: self.depth,
        }
    }

    /// Sets the depth stencil.
    pub fn depth_stencil_state(
        self,
        depth_stencil_state: wgpu::DepthStencilStateDescriptor,
    ) -> GlyphBrushBuilder<'a, wgpu::DepthStencilStateDescriptor, H> {
        GlyphBrushBuilder {
            inner: self.inner,
            texture_filter_method: self.texture_filter_method,
            depth: depth_stencil_state,
        }
    }
}

impl<'a, H: BuildHasher> GlyphBrushBuilder<'a, (), H> {
    /// Builds a `GlyphBrush` using the given `wgpu::Device` that can render
    /// text for texture views with the given `render_format`.
    pub fn build(
        self,
        device: &wgpu::Device,
        render_format: wgpu::TextureFormat,
    ) -> GlyphBrush<'a, (), H> {
        GlyphBrush::<(), H>::new(
            device,
            self.texture_filter_method,
            render_format,
            self.inner,
        )
    }
}

impl<'a, H: BuildHasher>
    GlyphBrushBuilder<'a, wgpu::DepthStencilStateDescriptor, H>
{
    /// Builds a `GlyphBrush` using the given `wgpu::Device` that can render
    /// text for texture views with the given `render_format`.
    pub fn build(
        self,
        device: &wgpu::Device,
        render_format: wgpu::TextureFormat,
    ) -> GlyphBrush<'a, wgpu::DepthStencilStateDescriptor, H> {
        GlyphBrush::<wgpu::DepthStencilStateDescriptor, H>::new(
            device,
            self.texture_filter_method,
            render_format,
            self.depth,
            self.inner,
        )
    }
}
