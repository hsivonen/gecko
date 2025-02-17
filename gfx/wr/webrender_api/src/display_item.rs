/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use euclid::{SideOffsets2D, TypedRect};
use std::ops::Not;
// local imports
use font;
use api::{PipelineId, PropertyBinding};
use color::ColorF;
use image::{ColorDepth, ImageKey};
use units::*;

// Maximum blur radius.
// Taken from nsCSSRendering.cpp in Gecko.
pub const MAX_BLUR_RADIUS: f32 = 300.;

// NOTE: some of these structs have an "IMPLICIT" comment.
// This indicates that the BuiltDisplayList will have serialized
// a list of values nearby that this item consumes. The traversal
// iterator should handle finding these.

/// A tag that can be used to identify items during hit testing. If the tag
/// is missing then the item doesn't take part in hit testing at all. This
/// is composed of two numbers. In Servo, the first is an identifier while the
/// second is used to select the cursor that should be used during mouse
/// movement. In Gecko, the first is a scrollframe identifier, while the second
/// is used to store various flags that APZ needs to properly process input
/// events.
pub type ItemTag = (u64, u16);

/// The DI is generic over the specifics, while allows to use
/// the "complete" version of it for convenient serialization.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct GenericDisplayItem<T> {
    pub item: T,
    pub layout: LayoutPrimitiveInfo,
    pub space_and_clip: SpaceAndClipInfo,
}

pub type DisplayItem = GenericDisplayItem<SpecificDisplayItem>;

/// A modified version of DI where every field is borrowed instead of owned.
/// It allows us to reduce copies during serialization.
#[derive(Serialize)]
pub struct SerializedDisplayItem<'a> {
    pub item: &'a SpecificDisplayItem,
    pub layout: &'a LayoutPrimitiveInfo,
    pub space_and_clip: &'a SpaceAndClipInfo,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct PrimitiveInfo<T> {
    pub rect: TypedRect<f32, T>,
    pub clip_rect: TypedRect<f32, T>,
    pub is_backface_visible: bool,
    pub tag: Option<ItemTag>,
}

impl LayoutPrimitiveInfo {
    pub fn new(rect: TypedRect<f32, LayoutPixel>) -> Self {
        Self::with_clip_rect(rect, rect)
    }

    pub fn with_clip_rect(
        rect: TypedRect<f32, LayoutPixel>,
        clip_rect: TypedRect<f32, LayoutPixel>,
    ) -> Self {
        PrimitiveInfo {
            rect,
            clip_rect,
            is_backface_visible: true,
            tag: None,
        }
    }
}

pub type LayoutPrimitiveInfo = PrimitiveInfo<LayoutPixel>;

/// Per-primitive information about the nodes in the clip tree and
/// the spatial tree that the primitive belongs to.
///
/// Note: this is a separate struct from `PrimitiveInfo` because
/// it needs indirectional mapping during the DL flattening phase,
/// turning into `ScrollNodeAndClipChain`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct SpaceAndClipInfo {
    pub spatial_id: SpatialId,
    pub clip_id: ClipId,
}

impl SpaceAndClipInfo {
    /// Create a new space/clip info associated with the root
    /// scroll frame.
    pub fn root_scroll(pipeline_id: PipelineId) -> Self {
        SpaceAndClipInfo {
            spatial_id: SpatialId::root_scroll_node(pipeline_id),
            clip_id: ClipId::root(pipeline_id),
        }
    }
}

#[repr(u64)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum SpecificDisplayItem {
    Clip(ClipDisplayItem),
    ScrollFrame(ScrollFrameDisplayItem),
    StickyFrame(StickyFrameDisplayItem),
    Rectangle(RectangleDisplayItem),
    ClearRectangle,
    Line(LineDisplayItem),
    Text(TextDisplayItem),
    Image(ImageDisplayItem),
    YuvImage(YuvImageDisplayItem),
    Border(BorderDisplayItem),
    BoxShadow(BoxShadowDisplayItem),
    Gradient(GradientDisplayItem),
    RadialGradient(RadialGradientDisplayItem),
    ClipChain(ClipChainItem),
    Iframe(IframeDisplayItem),
    PushReferenceFrame(ReferenceFrameDisplayListItem),
    PopReferenceFrame,
    PushStackingContext(PushStackingContextDisplayItem),
    PopStackingContext,
    SetGradientStops,
    PushShadow(Shadow),
    PopAllShadows,
    PushCacheMarker(CacheMarkerDisplayItem),
    PopCacheMarker,
    SetFilterOps,
    SetFilterData,
}

/// This is a "complete" version of the DI specifics,
/// containing the auxiliary data within the corresponding
/// enumeration variants, to be used for debug serialization.
#[cfg(any(feature = "serialize", feature = "deserialize"))]
#[cfg_attr(feature = "serialize", derive(Serialize))]
#[cfg_attr(feature = "deserialize", derive(Deserialize))]
pub enum CompletelySpecificDisplayItem {
    Clip(ClipDisplayItem, Vec<ComplexClipRegion>),
    ClipChain(ClipChainItem, Vec<ClipId>),
    ScrollFrame(ScrollFrameDisplayItem, Vec<ComplexClipRegion>),
    StickyFrame(StickyFrameDisplayItem),
    Rectangle(RectangleDisplayItem),
    ClearRectangle,
    Line(LineDisplayItem),
    Text(TextDisplayItem, Vec<font::GlyphInstance>),
    Image(ImageDisplayItem),
    YuvImage(YuvImageDisplayItem),
    Border(BorderDisplayItem),
    BoxShadow(BoxShadowDisplayItem),
    Gradient(GradientDisplayItem),
    RadialGradient(RadialGradientDisplayItem),
    Iframe(IframeDisplayItem),
    PushReferenceFrame(ReferenceFrameDisplayListItem),
    PopReferenceFrame,
    PushStackingContext(PushStackingContextDisplayItem),
    PopStackingContext,
    SetGradientStops(Vec<GradientStop>),
    PushShadow(Shadow),
    PopAllShadows,
    PushCacheMarker(CacheMarkerDisplayItem),
    PopCacheMarker,
    SetFilterOps(Vec<FilterOp>),
    SetFilterData(FilterData),
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ClipDisplayItem {
    pub id: ClipId,
    pub image_mask: Option<ImageMask>,
}

/// The minimum and maximum allowable offset for a sticky frame in a single dimension.
#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct StickyOffsetBounds {
    /// The minimum offset for this frame, typically a negative value, which specifies how
    /// far in the negative direction the sticky frame can offset its contents in this
    /// dimension.
    pub min: f32,

    /// The maximum offset for this frame, typically a positive value, which specifies how
    /// far in the positive direction the sticky frame can offset its contents in this
    /// dimension.
    pub max: f32,
}

impl StickyOffsetBounds {
    pub fn new(min: f32, max: f32) -> StickyOffsetBounds {
        StickyOffsetBounds { min, max }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct StickyFrameDisplayItem {
    pub id: SpatialId,

    /// The margins that should be maintained between the edge of the parent viewport and this
    /// sticky frame. A margin of None indicates that the sticky frame should not stick at all
    /// to that particular edge of the viewport.
    pub margins: SideOffsets2D<Option<f32>>,

    /// The minimum and maximum vertical offsets for this sticky frame. Ignoring these constraints,
    /// the sticky frame will continue to stick to the edge of the viewport as its original
    /// position is scrolled out of view. Constraints specify a maximum and minimum offset from the
    /// original position relative to non-sticky content within the same scrolling frame.
    pub vertical_offset_bounds: StickyOffsetBounds,

    /// The minimum and maximum horizontal offsets for this sticky frame. Ignoring these constraints,
    /// the sticky frame will continue to stick to the edge of the viewport as its original
    /// position is scrolled out of view. Constraints specify a maximum and minimum offset from the
    /// original position relative to non-sticky content within the same scrolling frame.
    pub horizontal_offset_bounds: StickyOffsetBounds,

    /// The amount of offset that has already been applied to the sticky frame. A positive y
    /// component this field means that a top-sticky item was in a scrollframe that has been
    /// scrolled down, such that the sticky item's position needed to be offset downwards by
    /// `previously_applied_offset.y`. A negative y component corresponds to the upward offset
    /// applied due to bottom-stickiness. The x-axis works analogously.
    pub previously_applied_offset: LayoutVector2D,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum ScrollSensitivity {
    ScriptAndInputEvents,
    Script,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ScrollFrameDisplayItem {
    pub clip_id: ClipId,
    pub scroll_frame_id: SpatialId,
    pub external_id: Option<ExternalScrollId>,
    pub image_mask: Option<ImageMask>,
    pub scroll_sensitivity: ScrollSensitivity,
    /// The amount this scrollframe has already been scrolled by, in the caller.
    /// This means that all the display items that are inside the scrollframe
    /// will have their coordinates shifted by this amount, and this offset
    /// should be added to those display item coordinates in order to get a
    /// normalized value that is consistent across display lists.
    pub external_scroll_offset: LayoutPoint,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct RectangleDisplayItem {
    pub color: ColorF,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct LineDisplayItem {
    pub orientation: LineOrientation, // toggles whether above values are interpreted as x/y values
    pub wavy_line_thickness: f32,
    pub color: ColorF,
    pub style: LineStyle,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Deserialize, MallocSizeOf, PartialEq, Serialize, Eq, Hash)]
pub enum LineOrientation {
    Vertical,
    Horizontal,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Deserialize, MallocSizeOf, PartialEq, Serialize, Eq, Hash)]
pub enum LineStyle {
    Solid,
    Dotted,
    Dashed,
    Wavy,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TextDisplayItem {
    pub font_key: font::FontInstanceKey,
    pub color: ColorF,
    pub glyph_options: Option<font::GlyphOptions>,
} // IMPLICIT: glyphs: Vec<font::GlyphInstance>

#[derive(Clone, Copy, Debug, Deserialize, MallocSizeOf, PartialEq, Serialize)]
pub struct NormalBorder {
    pub left: BorderSide,
    pub right: BorderSide,
    pub top: BorderSide,
    pub bottom: BorderSide,
    pub radius: BorderRadius,
    /// Whether to apply anti-aliasing on the border corners.
    ///
    /// Note that for this to be `false` and work, this requires the borders to
    /// be solid, and no border-radius.
    pub do_aa: bool,
}

impl NormalBorder {
    fn can_disable_antialiasing(&self) -> bool {
        fn is_valid(style: BorderStyle) -> bool {
            style == BorderStyle::Solid || style == BorderStyle::None
        }

        self.radius.is_zero() &&
            is_valid(self.top.style) &&
            is_valid(self.left.style) &&
            is_valid(self.bottom.style) &&
            is_valid(self.right.style)
    }

    /// Normalizes a border so that we don't render disallowed stuff, like inset
    /// borders that are less than two pixels wide.
    #[inline]
    pub fn normalize(&mut self, widths: &LayoutSideOffsets) {
        debug_assert!(
            self.do_aa || self.can_disable_antialiasing(),
            "Unexpected disabled-antialiasing in a border, likely won't work or will be ignored"
        );

        #[inline]
        fn renders_small_border_solid(style: BorderStyle) -> bool {
            match style {
                BorderStyle::Groove |
                BorderStyle::Ridge => true,
                _ => false,
            }
        }

        let normalize_side = |side: &mut BorderSide, width: f32| {
            if renders_small_border_solid(side.style) && width < 2. {
                side.style = BorderStyle::Solid;
            }
        };

        normalize_side(&mut self.left, widths.left);
        normalize_side(&mut self.right, widths.right);
        normalize_side(&mut self.top, widths.top);
        normalize_side(&mut self.bottom, widths.bottom);
    }
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, MallocSizeOf, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum RepeatMode {
    Stretch,
    Repeat,
    Round,
    Space,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum NinePatchBorderSource {
    Image(ImageKey),
    Gradient(Gradient),
    RadialGradient(RadialGradient),
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct NinePatchBorder {
    /// Describes what to use as the 9-patch source image. If this is an image,
    /// it will be stretched to fill the size given by width x height.
    pub source: NinePatchBorderSource,

    /// The width of the 9-part image.
    pub width: i32,

    /// The height of the 9-part image.
    pub height: i32,

    /// Distances from each edge where the image should be sliced up. These
    /// values are in 9-part-image space (the same space as width and height),
    /// and the resulting image parts will be used to fill the corresponding
    /// parts of the border as given by the border widths. This can lead to
    /// stretching.
    /// Slices can be overlapping. In that case, the same pixels from the
    /// 9-part image will show up in multiple parts of the resulting border.
    pub slice: SideOffsets2D<i32>,

    /// Controls whether the center of the 9 patch image is rendered or
    /// ignored. The center is never rendered if the slices are overlapping.
    pub fill: bool,

    /// Determines what happens if the horizontal side parts of the 9-part
    /// image have a different size than the horizontal parts of the border.
    pub repeat_horizontal: RepeatMode,

    /// Determines what happens if the vertical side parts of the 9-part
    /// image have a different size than the vertical parts of the border.
    pub repeat_vertical: RepeatMode,

    /// The outset for the border.
    /// TODO(mrobinson): This should be removed and handled by the client.
    pub outset: SideOffsets2D<f32>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum BorderDetails {
    Normal(NormalBorder),
    NinePatch(NinePatchBorder),
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct BorderDisplayItem {
    pub widths: LayoutSideOffsets,
    pub details: BorderDetails,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum BorderRadiusKind {
    Uniform,
    NonUniform,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, MallocSizeOf, PartialEq, Serialize)]
pub struct BorderRadius {
    pub top_left: LayoutSize,
    pub top_right: LayoutSize,
    pub bottom_left: LayoutSize,
    pub bottom_right: LayoutSize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, MallocSizeOf, PartialEq, Serialize)]
pub struct BorderSide {
    pub color: ColorF,
    pub style: BorderStyle,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Deserialize, MallocSizeOf, PartialEq, Serialize, Hash, Eq)]
pub enum BorderStyle {
    None = 0,
    Solid = 1,
    Double = 2,
    Dotted = 3,
    Dashed = 4,
    Hidden = 5,
    Groove = 6,
    Ridge = 7,
    Inset = 8,
    Outset = 9,
}

impl BorderStyle {
    pub fn is_hidden(&self) -> bool {
        *self == BorderStyle::Hidden || *self == BorderStyle::None
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, MallocSizeOf, PartialEq, Serialize)]
pub enum BoxShadowClipMode {
    Outset = 0,
    Inset = 1,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct BoxShadowDisplayItem {
    pub box_bounds: LayoutRect,
    pub offset: LayoutVector2D,
    pub color: ColorF,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub border_radius: BorderRadius,
    pub clip_mode: BoxShadowClipMode,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Shadow {
    pub offset: LayoutVector2D,
    pub color: ColorF,
    pub blur_radius: f32,
    pub should_inflate: bool,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Hash, Eq, MallocSizeOf, PartialEq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum ExtendMode {
    Clamp,
    Repeat,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Gradient {
    pub start_point: LayoutPoint,
    pub end_point: LayoutPoint,
    pub extend_mode: ExtendMode,
} // IMPLICIT: stops: Vec<GradientStop>

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct GradientDisplayItem {
    pub gradient: Gradient,
    pub tile_size: LayoutSize,
    pub tile_spacing: LayoutSize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, MallocSizeOf, PartialEq, Serialize)]
pub struct GradientStop {
    pub offset: f32,
    pub color: ColorF,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct RadialGradient {
    pub center: LayoutPoint,
    pub radius: LayoutSize,
    pub start_offset: f32,
    pub end_offset: f32,
    pub extend_mode: ExtendMode,
} // IMPLICIT stops: Vec<GradientStop>

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ClipChainItem {
    pub id: ClipChainId,
    pub parent: Option<ClipChainId>,
} // IMPLICIT stops: Vec<ClipId>

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct RadialGradientDisplayItem {
    pub gradient: RadialGradient,
    pub tile_size: LayoutSize,
    pub tile_spacing: LayoutSize,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ReferenceFrameDisplayListItem {
    pub reference_frame: ReferenceFrame,
}

/// Provides a hint to WR that it should try to cache the items
/// within a cache marker context in an off-screen surface.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct CacheMarkerDisplayItem {
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum ReferenceFrameKind {
    Transform,
    Perspective {
        scrolling_relative_to: Option<ExternalScrollId>,
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ReferenceFrame {
    pub kind: ReferenceFrameKind,
    pub transform_style: TransformStyle,
    /// The transform matrix, either the perspective matrix or the transform
    /// matrix.
    pub transform: PropertyBinding<LayoutTransform>,
    pub id: SpatialId,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct PushStackingContextDisplayItem {
    pub stacking_context: StackingContext,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct StackingContext {
    pub transform_style: TransformStyle,
    pub mix_blend_mode: MixBlendMode,
    pub clip_id: Option<ClipId>,
    pub raster_space: RasterSpace,
    /// True if picture caching should be used on this stacking context.
    pub cache_tiles: bool,
} // IMPLICIT: filters: Vec<FilterOp>, filter_datas: Vec<FilterData>

#[repr(u32)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum TransformStyle {
    Flat = 0,
    Preserve3D = 1,
}

/// Configure whether the contents of a stacking context
/// should be rasterized in local space or screen space.
/// Local space rasterized pictures are typically used
/// when we want to cache the output, and performance is
/// important. Note that this is a performance hint only,
/// which WR may choose to ignore.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[repr(u32)]
pub enum RasterSpace {
    // Rasterize in local-space, applying supplied scale to primitives.
    // Best performance, but lower quality.
    Local(f32),

    // Rasterize the picture in screen-space, including rotation / skew etc in
    // the rasterized element. Best quality, but slower performance. Note that
    // any stacking context with a perspective transform will be rasterized
    // in local-space, even if this is set.
    Screen,
}

impl RasterSpace {
    pub fn local_scale(&self) -> Option<f32> {
        match *self {
            RasterSpace::Local(scale) => Some(scale),
            RasterSpace::Screen => None,
        }
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum MixBlendMode {
    Normal = 0,
    Multiply = 1,
    Screen = 2,
    Overlay = 3,
    Darken = 4,
    Lighten = 5,
    ColorDodge = 6,
    ColorBurn = 7,
    HardLight = 8,
    SoftLight = 9,
    Difference = 10,
    Exclusion = 11,
    Hue = 12,
    Saturation = 13,
    Color = 14,
    Luminosity = 15,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum FilterOp {
    /// Filter that does no transformation of the colors, needed for
    /// debug purposes only.
    Identity,
    Blur(f32),
    Brightness(f32),
    Contrast(f32),
    Grayscale(f32),
    HueRotate(f32),
    Invert(f32),
    Opacity(PropertyBinding<f32>, f32),
    Saturate(f32),
    Sepia(f32),
    DropShadow(LayoutVector2D, f32, ColorF),
    ColorMatrix([f32; 20]),
    SrgbToLinear,
    LinearToSrgb,
    ComponentTransfer,
}

impl FilterOp {
    /// Ensure that the parameters for a filter operation
    /// are sensible.
    pub fn sanitize(self) -> FilterOp {
        match self {
            FilterOp::Blur(radius) => {
                let radius = radius.min(MAX_BLUR_RADIUS);
                FilterOp::Blur(radius)
            }
            FilterOp::DropShadow(offset, radius, color) => {
                let radius = radius.min(MAX_BLUR_RADIUS);
                FilterOp::DropShadow(offset, radius, color)
            }
            filter => filter,
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum ComponentTransferFuncType {
  Identity = 0,
  Table = 1,
  Discrete = 2,
  Linear = 3,
  Gamma = 4,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct FilterData {
    pub func_r_type: ComponentTransferFuncType,
    pub r_values: Vec<f32>,
    pub func_g_type: ComponentTransferFuncType,
    pub g_values: Vec<f32>,
    pub func_b_type: ComponentTransferFuncType,
    pub b_values: Vec<f32>,
    pub func_a_type: ComponentTransferFuncType,
    pub a_values: Vec<f32>,
}

fn sanitize_func_type(
    func_type: ComponentTransferFuncType,
    values: &[f32],
) -> ComponentTransferFuncType {
    if values.is_empty() {
        return ComponentTransferFuncType::Identity;
    }
    if values.len() < 2 && func_type == ComponentTransferFuncType::Linear {
        return ComponentTransferFuncType::Identity;
    }
    if values.len() < 3 && func_type == ComponentTransferFuncType::Gamma {
        return ComponentTransferFuncType::Identity;
    }
    func_type
}

fn sanitize_values(
    func_type: ComponentTransferFuncType,
    values: &[f32],
) -> bool {
    if values.len() < 2 && func_type == ComponentTransferFuncType::Linear {
        return false;
    }
    if values.len() < 3 && func_type == ComponentTransferFuncType::Gamma {
        return false;
    }
    true
}

impl FilterData {
    /// Ensure that the number of values matches up with the function type.
    pub fn sanitize(&self) -> FilterData {
        FilterData {
            func_r_type: sanitize_func_type(self.func_r_type, &self.r_values),
            r_values:
                    if sanitize_values(self.func_r_type, &self.r_values) {
                        self.r_values.clone()
                    } else {
                        Vec::new()
                    },
            func_g_type: sanitize_func_type(self.func_g_type, &self.g_values),
            g_values:
                    if sanitize_values(self.func_g_type, &self.g_values) {
                        self.g_values.clone()
                    } else {
                        Vec::new()
                    },

            func_b_type: sanitize_func_type(self.func_b_type, &self.b_values),
            b_values:
                    if sanitize_values(self.func_b_type, &self.b_values) {
                        self.b_values.clone()
                    } else {
                        Vec::new()
                    },

            func_a_type: sanitize_func_type(self.func_a_type, &self.a_values),
            a_values:
                    if sanitize_values(self.func_a_type, &self.a_values) {
                        self.a_values.clone()
                    } else {
                        Vec::new()
                    },

        }
    }

    pub fn is_identity(&self) -> bool {
        self.func_r_type == ComponentTransferFuncType::Identity &&
        self.func_g_type == ComponentTransferFuncType::Identity &&
        self.func_b_type == ComponentTransferFuncType::Identity &&
        self.func_a_type == ComponentTransferFuncType::Identity
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct IframeDisplayItem {
    pub pipeline_id: PipelineId,
    pub ignore_missing_pipeline: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ImageDisplayItem {
    pub image_key: ImageKey,
    pub stretch_size: LayoutSize,
    pub tile_spacing: LayoutSize,
    pub image_rendering: ImageRendering,
    pub alpha_type: AlphaType,
    pub color: ColorF,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, MallocSizeOf, PartialEq, Serialize)]
pub enum ImageRendering {
    Auto = 0,
    CrispEdges = 1,
    Pixelated = 2,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, MallocSizeOf, PartialEq, Serialize)]
pub enum AlphaType {
    Alpha = 0,
    PremultipliedAlpha = 1,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct YuvImageDisplayItem {
    pub yuv_data: YuvData,
    pub color_depth: ColorDepth,
    pub color_space: YuvColorSpace,
    pub image_rendering: ImageRendering,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, MallocSizeOf, PartialEq, Serialize)]
pub enum YuvColorSpace {
    Rec601 = 0,
    Rec709 = 1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum YuvData {
    NV12(ImageKey, ImageKey), // (Y channel, CbCr interleaved channel)
    PlanarYCbCr(ImageKey, ImageKey, ImageKey), // (Y channel, Cb channel, Cr Channel)
    InterleavedYCbCr(ImageKey), // (YCbCr interleaved channel)
}

impl YuvData {
    pub fn get_format(&self) -> YuvFormat {
        match *self {
            YuvData::NV12(..) => YuvFormat::NV12,
            YuvData::PlanarYCbCr(..) => YuvFormat::PlanarYCbCr,
            YuvData::InterleavedYCbCr(..) => YuvFormat::InterleavedYCbCr,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, MallocSizeOf, PartialEq, Serialize)]
pub enum YuvFormat {
    NV12 = 0,
    PlanarYCbCr = 1,
    InterleavedYCbCr = 2,
}

impl YuvFormat {
    pub fn get_plane_num(&self) -> usize {
        match *self {
            YuvFormat::NV12 => 2,
            YuvFormat::PlanarYCbCr => 3,
            YuvFormat::InterleavedYCbCr => 1,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ImageMask {
    pub image: ImageKey,
    pub rect: LayoutRect,
    pub repeat: bool,
}

impl ImageMask {
    /// Get a local clipping rect contributed by this mask.
    pub fn get_local_clip_rect(&self) -> Option<LayoutRect> {
        if self.repeat {
            None
        } else {
            Some(self.rect)
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, MallocSizeOf, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum ClipMode {
    Clip,    // Pixels inside the region are visible.
    ClipOut, // Pixels outside the region are visible.
}

impl Not for ClipMode {
    type Output = ClipMode;

    fn not(self) -> ClipMode {
        match self {
            ClipMode::Clip => ClipMode::ClipOut,
            ClipMode::ClipOut => ClipMode::Clip,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ComplexClipRegion {
    /// The boundaries of the rectangle.
    pub rect: LayoutRect,
    /// Border radii of this rectangle.
    pub radii: BorderRadius,
    /// Whether we are clipping inside or outside
    /// the region.
    pub mode: ClipMode,
}

impl BorderRadius {
    pub fn zero() -> BorderRadius {
        BorderRadius {
            top_left: LayoutSize::new(0.0, 0.0),
            top_right: LayoutSize::new(0.0, 0.0),
            bottom_left: LayoutSize::new(0.0, 0.0),
            bottom_right: LayoutSize::new(0.0, 0.0),
        }
    }

    pub fn uniform(radius: f32) -> BorderRadius {
        BorderRadius {
            top_left: LayoutSize::new(radius, radius),
            top_right: LayoutSize::new(radius, radius),
            bottom_left: LayoutSize::new(radius, radius),
            bottom_right: LayoutSize::new(radius, radius),
        }
    }

    pub fn uniform_size(radius: LayoutSize) -> BorderRadius {
        BorderRadius {
            top_left: radius,
            top_right: radius,
            bottom_left: radius,
            bottom_right: radius,
        }
    }

    pub fn is_uniform(&self) -> Option<f32> {
        match self.is_uniform_size() {
            Some(radius) if radius.width == radius.height => Some(radius.width),
            _ => None,
        }
    }

    pub fn is_uniform_size(&self) -> Option<LayoutSize> {
        let uniform_radius = self.top_left;
        if self.top_right == uniform_radius && self.bottom_left == uniform_radius &&
            self.bottom_right == uniform_radius
        {
            Some(uniform_radius)
        } else {
            None
        }
    }

    pub fn is_zero(&self) -> bool {
        if let Some(radius) = self.is_uniform() {
            radius == 0.0
        } else {
            false
        }
    }
}

impl ComplexClipRegion {
    /// Create a new complex clip region.
    pub fn new(
        rect: LayoutRect,
        radii: BorderRadius,
        mode: ClipMode,
    ) -> Self {
        ComplexClipRegion { rect, radii, mode }
    }
}

impl ComplexClipRegion {
    /// Get a local clipping rect contributed by this clip region.
    pub fn get_local_clip_rect(&self) -> Option<LayoutRect> {
        match self.mode {
            ClipMode::Clip => {
                Some(self.rect)
            }
            ClipMode::ClipOut => {
                None
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ClipChainId(pub u64, pub PipelineId);

/// A reference to a clipping node defining how an item is clipped.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ClipId {
    Clip(usize, PipelineId),
    ClipChain(ClipChainId),
}

const ROOT_CLIP_ID: usize = 0;

impl ClipId {
    /// Return the root clip ID - effectively doing no clipping.
    pub fn root(pipeline_id: PipelineId) -> Self {
        ClipId::Clip(ROOT_CLIP_ID, pipeline_id)
    }

    /// Return an invalid clip ID - needed in places where we carry
    /// one but need to not attempt to use it.
    pub fn invalid() -> Self {
        ClipId::Clip(!0, PipelineId::dummy())
    }

    pub fn pipeline_id(&self) -> PipelineId {
        match *self {
            ClipId::Clip(_, pipeline_id) |
            ClipId::ClipChain(ClipChainId(_, pipeline_id)) => pipeline_id,
        }
    }

    pub fn is_root(&self) -> bool {
        match *self {
            ClipId::Clip(id, _) => id == ROOT_CLIP_ID,
            ClipId::ClipChain(_) => false,
        }
    }

    pub fn is_valid(&self) -> bool {
        match *self {
            ClipId::Clip(id, _) => id != !0,
            _ => true,
        }
    }
}

/// A reference to a spatial node defining item positioning.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct SpatialId(pub usize, PipelineId);

const ROOT_REFERENCE_FRAME_SPATIAL_ID: usize = 0;
const ROOT_SCROLL_NODE_SPATIAL_ID: usize = 1;

impl SpatialId {
    pub fn new(spatial_node_index: usize, pipeline_id: PipelineId) -> Self {
        SpatialId(spatial_node_index, pipeline_id)
    }

    pub fn root_reference_frame(pipeline_id: PipelineId) -> Self {
        SpatialId(ROOT_REFERENCE_FRAME_SPATIAL_ID, pipeline_id)
    }

    pub fn root_scroll_node(pipeline_id: PipelineId) -> Self {
        SpatialId(ROOT_SCROLL_NODE_SPATIAL_ID, pipeline_id)
    }

    pub fn pipeline_id(&self) -> PipelineId {
        self.1
    }

    pub fn is_root_reference_frame(&self) -> bool {
        self.0 == ROOT_REFERENCE_FRAME_SPATIAL_ID
    }

    pub fn is_root_scroll_node(&self) -> bool {
        self.0 == ROOT_SCROLL_NODE_SPATIAL_ID
    }
}

/// An external identifier that uniquely identifies a scroll frame independent of its ClipId, which
/// may change from frame to frame. This should be unique within a pipeline. WebRender makes no
/// attempt to ensure uniqueness. The zero value is reserved for use by the root scroll node of
/// every pipeline, which always has an external id.
///
/// When setting display lists with the `preserve_frame_state` this id is used to preserve scroll
/// offsets between different sets of ClipScrollNodes which are ScrollFrames.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[repr(C)]
pub struct ExternalScrollId(pub u64, pub PipelineId);

impl ExternalScrollId {
    pub fn pipeline_id(&self) -> PipelineId {
        self.1
    }

    pub fn is_root(&self) -> bool {
        self.0 == 0
    }
}
