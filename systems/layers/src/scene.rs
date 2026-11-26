//! The ID scene: blitz-paint's calls drawn with the layer index of the node each draws for (blitz_paint::painting) as
//! a flat colour, antialiasing off, so each pixel holds exactly one layer's index: low byte red, high byte green,
//! alpha 255 where something was drawn. Brushes, opacity, blends and filters are not drawn; clips are. A node in no
//! layer draws index 0, so it still covers the layers under it.

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use anyrender::{Glyph, NormalizedCoord, PaintRef, PaintScene, RenderContext};
use blitz_dom::NodeId;
use vello_cpu::kurbo::{Affine, BezPath, Cap, Join, PathEl, Rect, Shape, Stroke};
use vello_cpu::peniko::{BlendMode, Color, Fill, FontData, StyleRef};
use vello_cpu::{PaintType, Pixmap, Resources};

/// a pixel more than this covered (of 255) is drawn whole, a less covered one not at all: about where an edge changes
/// the picture visibly (bench/layers/oracle.sh counts a change past 10 of 255)
const COVERED: u8 = 12;
const TOLERANCE: f64 = 0.1;

struct SceneCache {
    ctx: vello_cpu::RenderContext,
    resources: Resources,
    pixmap: Pixmap,
    last_key: Option<(u64, u16, u16)>,
    last_full: Option<Vec<u16>>,
    last_quarter: Option<Vec<u16>>,
}

thread_local! {
    static CACHE: RefCell<Option<SceneCache>> = const { RefCell::new(None) };
}

pub(crate) struct IdScene<'a> {
    ctx: Option<vello_cpu::RenderContext>,
    resources: Option<Resources>,
    pixmap: Option<Pixmap>,
    last_key: Option<(u64, u16, u16)>,
    last_full: Option<Vec<u16>>,
    last_quarter: Option<Vec<u16>>,
    /// each node's layer index (nodes not here: 0)
    ids: &'a HashMap<NodeId, u16>,
    hasher: DefaultHasher,
    w: u16,
    h: u16,
}

impl Drop for IdScene<'_> {
    fn drop(&mut self) {
        if let (Some(ctx), Some(resources), Some(pixmap)) = (self.ctx.take(), self.resources.take(), self.pixmap.take()) {
            let entry = SceneCache {
                ctx,
                resources,
                pixmap,
                last_key: self.last_key,
                last_full: self.last_full.take(),
                last_quarter: self.last_quarter.take(),
            };
            CACHE.with(|c| *c.borrow_mut() = Some(entry));
        }
    }
}

impl<'a> IdScene<'a> {
    pub(crate) fn new(w: u16, h: u16, ids: &'a HashMap<NodeId, u16>) -> Self {
        let cached = CACHE.with(|c| c.borrow_mut().take());
        let (mut ctx, resources, pixmap, last_key, last_full, last_quarter) = match cached {
            Some(c) if c.ctx.width() == w && c.ctx.height() == h => {
                (c.ctx, c.resources, c.pixmap, c.last_key, c.last_full, c.last_quarter)
            }
            Some(c) => (vello_cpu::RenderContext::new(w, h), c.resources, Pixmap::new(w, h), None, None, None),
            None => (vello_cpu::RenderContext::new(w, h), Resources::new(), Pixmap::new(w, h), None, None, None),
        };
        ctx.reset();
        ctx.set_aliasing_threshold(Some(COVERED));
        let mut hasher = DefaultHasher::new();
        (w, h).hash(&mut hasher);
        IdScene {
            ctx: Some(ctx),
            resources: Some(resources),
            pixmap: Some(pixmap),
            last_key,
            last_full,
            last_quarter,
            ids,
            hasher,
            w,
            h,
        }
    }

    /// Each pixel's layer index, row-major (NONE where nothing was drawn).
    pub(crate) fn finish(mut self) -> Vec<u16> {
        self.finish_with(false)
    }

    /// Quarter-size mask sampled at `(4x + 2, 4y + 2)` matching [`Recorder::frame`], avoiding a full-frame `Vec<u16>`.
    pub(crate) fn finish_quarter(mut self) -> Vec<u16> {
        self.finish_with(true)
    }

    fn finish_with(&mut self, quarter: bool) -> Vec<u16> {
        let hash = self.hasher.finish();
        let key = (hash, self.w, self.h);
        let hit = self.last_key == Some(key);
        if hit {
            if quarter && let Some(q) = &self.last_quarter {
                return q.clone();
            }
            if !quarter && let Some(f) = &self.last_full {
                return f.clone();
            }
            if quarter && let Some(f) = &self.last_full {
                let q = sample_quarter(self.w as u32, self.h as u32, |fx, fy| f[fy * self.w as usize + fx]);
                self.last_quarter = Some(q.clone());
                return q;
            }
        }
        let ctx = self.ctx.as_mut().unwrap();
        let res = self.resources.as_mut().unwrap();
        let px = self.pixmap.as_mut().unwrap();
        ctx.flush();
        ctx.render(&mut *px, res);
        let data = px.data();
        let decode = |p: vello_cpu::peniko::color::PremulRgba8| if p.a == 0 { crate::file::NONE } else { p.r as u16 | (p.g as u16) << 8 };
        let out = if quarter {
            sample_quarter(self.w as u32, self.h as u32, |fx, fy| decode(data[fy * self.w as usize + fx]))
        } else {
            data.iter().copied().map(decode).collect()
        };
        if !hit {
            self.last_full = None;
            self.last_quarter = None;
            self.last_key = Some(key);
        }
        if quarter {
            self.last_quarter = Some(out.clone());
        } else {
            self.last_full = Some(out.clone());
        }
        out
    }

    /// Ready to draw for the node being painted, at `transform`: false when its brush draws nothing.
    fn brush(&mut self, paint: PaintRef<'_>, transform: Affine) -> bool {
        if let PaintRef::Solid(c) = paint {
            if c.components[3] <= 0.0 {
                return false;
            }
        }
        let id = blitz_paint::painting().and_then(|n| self.ids.get(&n)).copied().unwrap_or(0);
        id.hash(&mut self.hasher);
        hash_affine(&mut self.hasher, transform);
        let ctx = self.ctx.as_mut().unwrap();
        ctx.set_transform(transform);
        ctx.reset_paint_transform();
        ctx.set_paint(PaintType::Solid(Color::from_rgba8((id & 0xff) as u8, (id >> 8) as u8, 0, 255)));
        true
    }
}

pub(crate) fn paint_quarter(doc: &mut blitz_dom::BaseDocument, scale: f64, w: u32, h: u32, ids: &HashMap<NodeId, u16>) -> Vec<u16> {
    let mut scene = IdScene::new(w.min(u16::MAX as u32) as u16, h.min(u16::MAX as u32) as u16, ids);
    blitz_paint::paint_scene(&mut scene, doc, scale, w, h, 0, 0);
    scene.finish_quarter()
}

fn sample_quarter(w: u32, h: u32, mut at: impl FnMut(usize, usize) -> u16) -> Vec<u16> {
    let scale = crate::file::SCALE;
    let (mw, mh) = (w.div_ceil(scale), h.div_ceil(scale));
    let mut out = Vec::with_capacity((mw * mh) as usize);
    for y in 0..mh {
        let fy = (y * scale + scale / 2).min(h.saturating_sub(1)) as usize;
        for x in 0..mw {
            let fx = (x * scale + scale / 2).min(w.saturating_sub(1)) as usize;
            out.push(at(fx, fy));
        }
    }
    out
}

fn hash_affine(h: &mut DefaultHasher, a: Affine) {
    for v in a.as_coeffs() {
        v.to_bits().hash(h);
    }
}

fn hash_fill(h: &mut DefaultHasher, f: Fill) {
    matches!(f, Fill::EvenOdd).hash(h);
}

fn hash_cap(h: &mut DefaultHasher, c: Cap) {
    match c {
        Cap::Butt => 0u8,
        Cap::Square => 1u8,
        Cap::Round => 2u8,
    }
    .hash(h);
}

fn hash_stroke(h: &mut DefaultHasher, s: &Stroke) {
    s.width.to_bits().hash(h);
    match s.join {
        Join::Bevel => 0u8,
        Join::Miter => 1u8,
        Join::Round => 2u8,
    }
    .hash(h);
    s.miter_limit.to_bits().hash(h);
    hash_cap(h, s.start_cap);
    hash_cap(h, s.end_cap);
    s.dash_offset.to_bits().hash(h);
    s.dash_pattern.len().hash(h);
    for d in &s.dash_pattern {
        d.to_bits().hash(h);
    }
}

fn hash_path(h: &mut DefaultHasher, path: &BezPath) {
    path.elements().len().hash(h);
    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => { 0u8.hash(h); p.x.to_bits().hash(h); p.y.to_bits().hash(h); }
            PathEl::LineTo(p) => { 1u8.hash(h); p.x.to_bits().hash(h); p.y.to_bits().hash(h); }
            PathEl::QuadTo(p1, p2) => {
                2u8.hash(h);
                for p in [p1, p2] { p.x.to_bits().hash(h); p.y.to_bits().hash(h); }
            }
            PathEl::CurveTo(p1, p2, p3) => {
                3u8.hash(h);
                for p in [p1, p2, p3] { p.x.to_bits().hash(h); p.y.to_bits().hash(h); }
            }
            PathEl::ClosePath => 4u8.hash(h),
        }
    }
}

impl RenderContext for IdScene<'_> {}

impl PaintScene for IdScene<'_> {
    fn reset(&mut self) {
        let ctx = self.ctx.as_mut().unwrap();
        ctx.reset();
        ctx.set_aliasing_threshold(Some(COVERED));
        self.hasher = DefaultHasher::new();
        (self.w, self.h).hash(&mut self.hasher);
    }

    fn push_layer(&mut self, fill: Fill, _blend: impl Into<BlendMode>, alpha: f32, transform: Affine, clip: &impl Shape, _filter: Option<Arc<anyrender::Filter>>, _backdrop: Option<Arc<anyrender::Filter>>) {
        // a layer drawn at no opacity shows nothing of its content
        if alpha <= 0.0 {
            return self.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &Rect::ZERO);
        }
        self.push_clip_layer(fill, transform, clip);
    }

    fn push_clip_layer(&mut self, fill: Fill, transform: Affine, clip: &impl Shape) {
        let path = clip.into_path(TOLERANCE);
        1u8.hash(&mut self.hasher);
        hash_fill(&mut self.hasher, fill);
        hash_affine(&mut self.hasher, transform);
        hash_path(&mut self.hasher, &path);
        let ctx = self.ctx.as_mut().unwrap();
        ctx.set_transform(transform);
        ctx.set_fill_rule(fill);
        ctx.push_clip_layer(&path);
    }

    fn pop_layer(&mut self) {
        2u8.hash(&mut self.hasher);
        self.ctx.as_mut().unwrap().pop_layer();
    }

    fn stroke<'b>(&mut self, style: &Stroke, transform: Affine, brush: impl Into<PaintRef<'b>>, _bt: Option<Affine>, shape: &impl Shape) {
        if self.brush(brush.into(), transform) {
            let path = shape.into_path(TOLERANCE);
            3u8.hash(&mut self.hasher);
            hash_stroke(&mut self.hasher, style);
            hash_path(&mut self.hasher, &path);
            let ctx = self.ctx.as_mut().unwrap();
            ctx.set_stroke(style.clone());
            ctx.stroke_path(&path);
        }
    }

    fn fill<'b>(&mut self, style: Fill, transform: Affine, brush: impl Into<PaintRef<'b>>, _bt: Option<Affine>, shape: &impl Shape) {
        if !self.brush(brush.into(), transform) {
            return;
        }
        let path = shape.into_path(TOLERANCE);
        4u8.hash(&mut self.hasher);
        matches!(style, Fill::EvenOdd).hash(&mut self.hasher);
        hash_path(&mut self.hasher, &path);
        let ctx = self.ctx.as_mut().unwrap();
        ctx.set_fill_rule(style);
        ctx.fill_path(&path);
    }

    fn draw_glyphs<'b, 's: 'b>(
        &'s mut self,
        font: &'b FontData,
        font_size: f32,
        hint: bool,
        coords: &'b [NormalizedCoord],
        embolden: vello_cpu::kurbo::Vec2,
        style: impl Into<StyleRef<'b>>,
        brush: impl Into<PaintRef<'b>>,
        brush_alpha: f32,
        transform: Affine,
        glyph_transform: Option<Affine>,
        glyphs: impl Iterator<Item = Glyph> + Clone,
    ) {
        if brush_alpha <= 0.0 || !self.brush(brush.into(), transform) {
            return;
        }
        let style = style.into();
        5u8.hash(&mut self.hasher);
        (font.data.id(), font.index, font_size.to_bits(), hint, coords, embolden.x.to_bits(), embolden.y.to_bits()).hash(&mut self.hasher);
        glyph_transform.is_some().hash(&mut self.hasher);
        if let Some(gt) = glyph_transform {
            hash_affine(&mut self.hasher, gt);
        }
        for g in glyphs.clone() {
            (g.id, g.x.to_bits(), g.y.to_bits()).hash(&mut self.hasher);
        }
        let glyphs = glyphs.map(|g| vello_cpu::Glyph { id: g.id, x: g.x, y: g.y });
        let ctx = self.ctx.as_mut().unwrap();
        let res = self.resources.as_mut().unwrap();
        match style {
            StyleRef::Fill(fill) => {
                0u8.hash(&mut self.hasher);
                hash_fill(&mut self.hasher, fill);
                ctx.set_fill_rule(fill);
                ctx.glyph_run(res, font).font_size(font_size).hint(hint).normalized_coords(coords).glyph_transform(glyph_transform.unwrap_or_default()).font_embolden(glifo::FontEmbolden::new(vello_cpu::kurbo::Diagonal2::new(embolden.x, embolden.y))).fill_glyphs(glyphs);
            }
            StyleRef::Stroke(stroke) => {
                1u8.hash(&mut self.hasher);
                hash_stroke(&mut self.hasher, stroke);
                ctx.set_stroke(stroke.clone());
                ctx.glyph_run(res, font).font_size(font_size).hint(hint).normalized_coords(coords).glyph_transform(glyph_transform.unwrap_or_default()).font_embolden(glifo::FontEmbolden::new(vello_cpu::kurbo::Diagonal2::new(embolden.x, embolden.y))).stroke_glyphs(glyphs);
            }
        }
    }

    /// a shadow is its element's (hiding the element hides it): drawn out to where its blur still shows, about 1.5
    /// deviations past its box
    fn draw_box_shadow(&mut self, transform: Affine, rect: Rect, brush: Color, radius: f64, std_dev: f64) {
        if self.brush(PaintRef::Solid(brush), transform) {
            6u8.hash(&mut self.hasher);
            for v in [rect.x0, rect.y0, rect.x1, rect.y1, radius, std_dev] {
                v.to_bits().hash(&mut self.hasher);
            }
            let grow = 1.5 * std_dev;
            self.ctx.as_mut().unwrap().fill_path(&vello_cpu::kurbo::RoundedRect::from_rect(rect.inflate(grow, grow), radius + grow).into_path(TOLERANCE));
        }
    }
}
