/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use api::{ExternalScrollId, LayoutPoint, LayoutRect, LayoutVector2D, ReferenceFrameKind};
use api::{PipelineId, ScrollClamping, ScrollNodeState, ScrollLocation, ScrollSensitivity};
use api::{LayoutSize, LayoutTransform, PropertyBinding, TransformStyle, WorldPoint};
use gpu_types::TransformPalette;
use internal_types::{FastHashMap, FastHashSet};
use print_tree::{PrintableTree, PrintTree, PrintTreePrinter};
use scene::SceneProperties;
use spatial_node::{ScrollFrameInfo, SpatialNode, SpatialNodeType, StickyFrameInfo, ScrollFrameKind};
use std::ops;
use util::{project_rect, LayoutToWorldFastTransform, MatrixHelpers, ScaleOffset};

pub type ScrollStates = FastHashMap<ExternalScrollId, ScrollFrameInfo>;

/// An id that identifies coordinate systems in the ClipScrollTree. Each
/// coordinate system has an id and those ids will be shared when the coordinates
/// system are the same or are in the same axis-aligned space. This allows
/// for optimizing mask generation.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "capture", derive(Serialize))]
#[cfg_attr(feature = "replay", derive(Deserialize))]
pub struct CoordinateSystemId(pub u32);

/// A node in the hierarchy of coordinate system
/// transforms.
#[derive(Debug)]
pub struct CoordinateSystem {
    pub transform: LayoutTransform,
    /// True if the Z component of the resulting transform, when ascending
    /// from children to a parent, needs to be flattened upon passing this system.
    pub is_flatten_root: bool,
    pub parent: Option<CoordinateSystemId>,
}

impl CoordinateSystem {
    fn root() -> Self {
        CoordinateSystem {
            transform: LayoutTransform::identity(),
            is_flatten_root: true,
            parent: None,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, Hash, MallocSizeOf, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "capture", derive(Serialize))]
#[cfg_attr(feature = "replay", derive(Deserialize))]
pub struct SpatialNodeIndex(pub u32);

//Note: these have to match ROOT_REFERENCE_FRAME_SPATIAL_ID and ROOT_SCROLL_NODE_SPATIAL_ID
pub const ROOT_SPATIAL_NODE_INDEX: SpatialNodeIndex = SpatialNodeIndex(0);
const TOPMOST_SCROLL_NODE_INDEX: SpatialNodeIndex = SpatialNodeIndex(1);

impl SpatialNodeIndex {
    pub fn new(index: usize) -> Self {
        debug_assert!(index < ::std::u32::MAX as usize);
        SpatialNodeIndex(index as u32)
    }
}

impl CoordinateSystemId {
    pub fn root() -> Self {
        CoordinateSystemId(0)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum VisibleFace {
    Front,
    Back,
}

impl Default for VisibleFace {
    fn default() -> Self {
        VisibleFace::Front
    }
}

impl ops::Not for VisibleFace {
    type Output = Self;
    fn not(self) -> Self {
        match self {
            VisibleFace::Front => VisibleFace::Back,
            VisibleFace::Back => VisibleFace::Front,
        }
    }
}

pub struct ClipScrollTree {
    /// Nodes which determine the positions (offsets and transforms) for primitives
    /// and clips.
    pub spatial_nodes: Vec<SpatialNode>,

    /// A list of transforms that establish new coordinate systems.
    /// Spatial nodes only establish a new coordinate system when
    /// they have a transform that is not a simple 2d translation.
    coord_systems: Vec<CoordinateSystem>,

    pub pending_scroll_offsets: FastHashMap<ExternalScrollId, (LayoutPoint, ScrollClamping)>,

    /// A set of pipelines which should be discarded the next time this
    /// tree is drained.
    pub pipelines_to_discard: FastHashSet<PipelineId>,

    /// Temporary stack of nodes to update when traversing the tree.
    nodes_to_update: Vec<(SpatialNodeIndex, TransformUpdateState)>,
}

#[derive(Clone)]
pub struct TransformUpdateState {
    pub parent_reference_frame_transform: LayoutToWorldFastTransform,
    pub parent_accumulated_scroll_offset: LayoutVector2D,
    pub nearest_scrolling_ancestor_offset: LayoutVector2D,
    pub nearest_scrolling_ancestor_viewport: LayoutRect,

    /// An id for keeping track of the axis-aligned space of this node. This is used in
    /// order to to track what kinds of clip optimizations can be done for a particular
    /// display list item, since optimizations can usually only be done among
    /// coordinate systems which are relatively axis aligned.
    pub current_coordinate_system_id: CoordinateSystemId,

    /// Scale and offset from the coordinate system that started this compatible coordinate system.
    pub coordinate_system_relative_scale_offset: ScaleOffset,

    /// True if this node is transformed by an invertible transform.  If not, display items
    /// transformed by this node will not be displayed and display items not transformed by this
    /// node will not be clipped by clips that are transformed by this node.
    pub invertible: bool,

    /// True if this node is a part of Preserve3D hierarchy.
    pub preserves_3d: bool,
}

/// A processed relative transform between two nodes in the clip-scroll tree.
#[derive(Debug, Default)]
pub struct RelativeTransform {
    /// The flattened transform, produces Z = 0 at all times.
    pub flattened: LayoutTransform,
    /// Visible face of the original transform.
    pub visible_face: VisibleFace,
    /// True if the original transform had perspective.
    pub is_perspective: bool,
}

impl ClipScrollTree {
    pub fn new() -> Self {
        ClipScrollTree {
            spatial_nodes: Vec::new(),
            coord_systems: Vec::new(),
            pending_scroll_offsets: FastHashMap::default(),
            pipelines_to_discard: FastHashSet::default(),
            nodes_to_update: Vec::new(),
        }
    }

    /// Calculate the relative transform from `child_index` to `parent_index`.
    /// This method will panic if the nodes are not connected!
    pub fn get_relative_transform(
        &self,
        child_index: SpatialNodeIndex,
        parent_index: SpatialNodeIndex,
    ) -> Option<RelativeTransform> {
        assert!(child_index.0 >= parent_index.0);
        let child = &self.spatial_nodes[child_index.0 as usize];
        let parent = &self.spatial_nodes[parent_index.0 as usize];

        let mut coordinate_system_id = child.coordinate_system_id;
        let mut transform = child.coordinate_system_relative_scale_offset.to_transform();
        let mut visible_face = VisibleFace::Front;
        let mut is_perspective = false;

        while coordinate_system_id != parent.coordinate_system_id {
            let coord_system = &self.coord_systems[coordinate_system_id.0 as usize];
            coordinate_system_id = coord_system.parent.expect("invalid parent!");
            transform = transform.post_mul(&coord_system.transform);
            // we need to update the associated parameters of a transform in two cases:
            // 1) when the flattening happens, so that we don't lose that original 3D aspects
            // 2) when we reach the end of iteration, so that our result is up to date
            if coord_system.is_flatten_root || coordinate_system_id == parent.coordinate_system_id {
                visible_face = if transform.is_backface_visible() {
                    VisibleFace::Back
                } else {
                    VisibleFace::Front
                };
                is_perspective = transform.has_perspective_component();
            }
            if coord_system.is_flatten_root {
                //Note: this function makes the transform to ignore the Z coordinate of inputs
                // *even* for computing the X and Y coordinates of the output.
                //transform = transform.project_to_2d();
                transform.m13 = 0.0;
                transform.m23 = 0.0;
                transform.m33 = 0.0;
                transform.m43 = 0.0;
            }
        }

        transform = transform.post_mul(
            &parent.coordinate_system_relative_scale_offset
                .inverse()
                .to_transform()
        );

        Some(RelativeTransform {
            flattened: transform,
            visible_face,
            is_perspective,
        })
    }

    /// Map a rectangle in some child space to a parent.
    /// Doesn't handle preserve-3d islands.
    pub fn map_rect_to_parent_space(
        &self,
        mut rect: LayoutRect,
        child_index: SpatialNodeIndex,
        parent_index: SpatialNodeIndex,
        parent_bounds: &LayoutRect,
    ) -> Option<LayoutRect> {
        if child_index == parent_index {
            return Some(rect);
        }
        assert!(child_index.0 > parent_index.0);

        let child = &self.spatial_nodes[child_index.0 as usize];
        let parent = &self.spatial_nodes[parent_index.0 as usize];

        let mut coordinate_system_id = child.coordinate_system_id;
        rect = child.coordinate_system_relative_scale_offset.map_rect(&rect);

        while coordinate_system_id != parent.coordinate_system_id {
            let coord_system = &self.coord_systems[coordinate_system_id.0 as usize];
            coordinate_system_id = coord_system.parent.expect("invalid parent!");
            rect = project_rect(&coord_system.transform, &rect, parent_bounds)?;
        }

        Some(parent.coordinate_system_relative_scale_offset.unmap_rect(&rect))
    }

    /// Returns true if the spatial node is the same as the parent, or is
    /// a child of the parent.
    pub fn is_same_or_child_of(
        &self,
        spatial_node_index: SpatialNodeIndex,
        parent_spatial_node_index: SpatialNodeIndex,
    ) -> bool {
        let mut index = spatial_node_index;

        loop {
            if index == parent_spatial_node_index {
                return true;
            }

            index = match self.spatial_nodes[index.0 as usize].parent {
                Some(parent) => parent,
                None => return false,
            }
        }
    }

    /// The root reference frame, which is the true root of the ClipScrollTree. Initially
    /// this ID is not valid, which is indicated by ```spatial_nodes``` being empty.
    pub fn root_reference_frame_index(&self) -> SpatialNodeIndex {
        // TODO(mrobinson): We should eventually make this impossible to misuse.
        debug_assert!(!self.spatial_nodes.is_empty());
        ROOT_SPATIAL_NODE_INDEX
    }

    /// The root scroll node which is the first child of the root reference frame.
    /// Initially this ID is not valid, which is indicated by ```spatial_nodes``` being empty.
    pub fn topmost_scroll_node_index(&self) -> SpatialNodeIndex {
        // TODO(mrobinson): We should eventually make this impossible to misuse.
        debug_assert!(self.spatial_nodes.len() >= 1);
        TOPMOST_SCROLL_NODE_INDEX
    }

    pub fn get_scroll_node_state(&self) -> Vec<ScrollNodeState> {
        let mut result = vec![];
        for node in &self.spatial_nodes {
            if let SpatialNodeType::ScrollFrame(info) = node.node_type {
                if let Some(id) = info.external_id {
                    result.push(ScrollNodeState { id, scroll_offset: info.offset })
                }
            }
        }
        result
    }

    pub fn drain(&mut self) -> ScrollStates {
        let mut scroll_states = FastHashMap::default();
        for old_node in &mut self.spatial_nodes.drain(..) {
            if self.pipelines_to_discard.contains(&old_node.pipeline_id) {
                continue;
            }

            match old_node.node_type {
                SpatialNodeType::ScrollFrame(info) if info.external_id.is_some() => {
                    scroll_states.insert(info.external_id.unwrap(), info);
                }
                _ => {}
            }
        }

        self.coord_systems.clear();
        self.pipelines_to_discard.clear();
        scroll_states
    }

    pub fn scroll_node(
        &mut self,
        origin: LayoutPoint,
        id: ExternalScrollId,
        clamp: ScrollClamping
    ) -> bool {
        for node in &mut self.spatial_nodes {
            if node.matches_external_id(id) {
                return node.set_scroll_origin(&origin, clamp);
            }
        }

        self.pending_scroll_offsets.insert(id, (origin, clamp));
        false
    }

    fn find_nearest_scrolling_ancestor(
        &self,
        index: Option<SpatialNodeIndex>
    ) -> SpatialNodeIndex {
        let index = match index {
            Some(index) => index,
            None => return self.topmost_scroll_node_index(),
        };

        let node = &self.spatial_nodes[index.0 as usize];
        match node.node_type {
            SpatialNodeType::ScrollFrame(state) if state.sensitive_to_input_events() => index,
            _ => self.find_nearest_scrolling_ancestor(node.parent)
        }
    }

    pub fn scroll_nearest_scrolling_ancestor(
        &mut self,
        scroll_location: ScrollLocation,
        node_index: Option<SpatialNodeIndex>,
    ) -> bool {
        if self.spatial_nodes.is_empty() {
            return false;
        }
        let node_index = self.find_nearest_scrolling_ancestor(node_index);
        self.spatial_nodes[node_index.0 as usize].scroll(scroll_location)
    }

    pub fn update_tree(
        &mut self,
        pan: WorldPoint,
        scene_properties: &SceneProperties,
        mut transform_palette: Option<&mut TransformPalette>,
    ) {
        if self.spatial_nodes.is_empty() {
            return;
        }

        if let Some(ref mut palette) = transform_palette {
            palette.allocate(self.spatial_nodes.len());
        }

        self.coord_systems.clear();
        self.coord_systems.push(CoordinateSystem::root());

        let root_node_index = self.root_reference_frame_index();
        let state = TransformUpdateState {
            parent_reference_frame_transform: LayoutVector2D::new(pan.x, pan.y).into(),
            parent_accumulated_scroll_offset: LayoutVector2D::zero(),
            nearest_scrolling_ancestor_offset: LayoutVector2D::zero(),
            nearest_scrolling_ancestor_viewport: LayoutRect::zero(),
            current_coordinate_system_id: CoordinateSystemId::root(),
            coordinate_system_relative_scale_offset: ScaleOffset::identity(),
            invertible: true,
            preserves_3d: false,
        };
        debug_assert!(self.nodes_to_update.is_empty());
        self.nodes_to_update.push((root_node_index, state));

        while let Some((node_index, mut state)) = self.nodes_to_update.pop() {
            let (previous, following) = self.spatial_nodes.split_at_mut(node_index.0 as usize);
            let node = match following.get_mut(0) {
                Some(node) => node,
                None => continue,
            };

            node.update(&mut state, &mut self.coord_systems, scene_properties, &*previous);
            if let Some(ref mut palette) = transform_palette {
                node.push_gpu_data(palette, node_index);
            }

            if !node.children.is_empty() {
                node.prepare_state_for_children(&mut state);
                self.nodes_to_update.extend(node.children
                    .iter()
                    .rev()
                    .map(|child_index| (*child_index, state.clone()))
                );
            }
        }
    }

    pub fn finalize_and_apply_pending_scroll_offsets(&mut self, old_states: ScrollStates) {
        for node in &mut self.spatial_nodes {
            let external_id = match node.node_type {
                SpatialNodeType::ScrollFrame(ScrollFrameInfo { external_id: Some(id), ..} ) => id,
                _ => continue,
            };

            if let Some(scrolling_state) = old_states.get(&external_id) {
                node.apply_old_scrolling_state(scrolling_state);
            }

            if let Some((offset, clamping)) = self.pending_scroll_offsets.remove(&external_id) {
                node.set_scroll_origin(&offset, clamping);
            }
        }
    }

    pub fn add_scroll_frame(
        &mut self,
        parent_index: SpatialNodeIndex,
        external_id: Option<ExternalScrollId>,
        pipeline_id: PipelineId,
        frame_rect: &LayoutRect,
        content_size: &LayoutSize,
        scroll_sensitivity: ScrollSensitivity,
        frame_kind: ScrollFrameKind,
    ) -> SpatialNodeIndex {
        let node = SpatialNode::new_scroll_frame(
            pipeline_id,
            parent_index,
            external_id,
            frame_rect,
            content_size,
            scroll_sensitivity,
            frame_kind,
        );
        self.add_spatial_node(node)
    }

    pub fn add_reference_frame(
        &mut self,
        parent_index: Option<SpatialNodeIndex>,
        transform_style: TransformStyle,
        source_transform: PropertyBinding<LayoutTransform>,
        kind: ReferenceFrameKind,
        origin_in_parent_reference_frame: LayoutVector2D,
        pipeline_id: PipelineId,
    ) -> SpatialNodeIndex {
        let node = SpatialNode::new_reference_frame(
            parent_index,
            transform_style,
            source_transform,
            kind,
            origin_in_parent_reference_frame,
            pipeline_id,
        );
        self.add_spatial_node(node)
    }

    pub fn add_sticky_frame(
        &mut self,
        parent_index: SpatialNodeIndex,
        sticky_frame_info: StickyFrameInfo,
        pipeline_id: PipelineId,
    ) -> SpatialNodeIndex {
        let node = SpatialNode::new_sticky_frame(
            parent_index,
            sticky_frame_info,
            pipeline_id,
        );
        self.add_spatial_node(node)
    }

    pub fn add_spatial_node(&mut self, node: SpatialNode) -> SpatialNodeIndex {
        let index = SpatialNodeIndex::new(self.spatial_nodes.len());

        // When the parent node is None this means we are adding the root.
        if let Some(parent_index) = node.parent {
            self.spatial_nodes[parent_index.0 as usize].add_child(index);
        }

        self.spatial_nodes.push(node);
        index
    }

    pub fn discard_frame_state_for_pipeline(&mut self, pipeline_id: PipelineId) {
        self.pipelines_to_discard.insert(pipeline_id);
    }

    fn print_node<T: PrintTreePrinter>(
        &self,
        index: SpatialNodeIndex,
        pt: &mut T,
    ) {
        let node = &self.spatial_nodes[index.0 as usize];
        match node.node_type {
            SpatialNodeType::StickyFrame(ref sticky_frame_info) => {
                pt.new_level(format!("StickyFrame"));
                pt.add_item(format!("index: {:?}", index));
                pt.add_item(format!("sticky info: {:?}", sticky_frame_info));
            }
            SpatialNodeType::ScrollFrame(scrolling_info) => {
                pt.new_level(format!("ScrollFrame"));
                pt.add_item(format!("index: {:?}", index));
                pt.add_item(format!("viewport: {:?}", scrolling_info.viewport_rect));
                pt.add_item(format!("scrollable_size: {:?}", scrolling_info.scrollable_size));
                pt.add_item(format!("scroll offset: {:?}", scrolling_info.offset));
            }
            SpatialNodeType::ReferenceFrame(ref _info) => {
                pt.new_level(format!("ReferenceFrame"));
                pt.add_item(format!("index: {:?}", index));
            }
        }

        pt.add_item(format!("world_viewport_transform: {:?}", node.world_viewport_transform));
        pt.add_item(format!("world_content_transform: {:?}", node.world_content_transform));
        pt.add_item(format!("coordinate_system_id: {:?}", node.coordinate_system_id));

        for child_index in &node.children {
            self.print_node(*child_index, pt);
        }

        pt.end_level();
    }

    #[allow(dead_code)]
    pub fn print(&self) {
        if !self.spatial_nodes.is_empty() {
            let mut pt = PrintTree::new("clip_scroll tree");
            self.print_with(&mut pt);
        }
    }

    /// Return true if this is a guaranteed identity transform. This
    /// is conservative, it assumes not identity if a property
    /// binding animation, or scroll frame is found, for example.
    pub fn node_is_identity(
        &self,
        spatial_node_index: SpatialNodeIndex,
    ) -> bool {
        let mut current = spatial_node_index;

        while current != ROOT_SPATIAL_NODE_INDEX {
            let node = &self.spatial_nodes[current.0 as usize];

            match node.node_type {
                SpatialNodeType::ReferenceFrame(ref info) => {
                    match info.source_transform {
                        PropertyBinding::Value(transform) => {
                            if transform != LayoutTransform::identity() {
                                return false;
                            }
                        }
                        PropertyBinding::Binding(..) => {
                            // Assume not identity since it may change with animation.
                            return false;
                        }
                    }
                }
                SpatialNodeType::ScrollFrame(ref info) => {
                    // Assume not identity since it may change with scrolling.
                    if let ScrollFrameKind::Explicit = info.frame_kind {
                        return false;
                    }
                }
                SpatialNodeType::StickyFrame(..) => {
                    // Assume not identity since it may change with scrolling.
                    return false;
                }
            }
            current = node.parent.unwrap();
        }

        true
    }
}

impl PrintableTree for ClipScrollTree {
    fn print_with<T: PrintTreePrinter>(&self, pt: &mut T) {
        if !self.spatial_nodes.is_empty() {
            self.print_node(self.root_reference_frame_index(), pt);
        }
    }
}

#[cfg(test)]
fn add_reference_frame(
    cst: &mut ClipScrollTree,
    parent: Option<SpatialNodeIndex>,
    transform: LayoutTransform,
    origin_in_parent_reference_frame: LayoutVector2D,
) -> SpatialNodeIndex {
    cst.add_reference_frame(
        parent,
        TransformStyle::Preserve3D,
        PropertyBinding::Value(transform),
        ReferenceFrameKind::Transform,
        origin_in_parent_reference_frame,
        PipelineId::dummy(),
    )
}

#[cfg(test)]
fn test_pt(
    px: f32,
    py: f32,
    cst: &ClipScrollTree,
    child: SpatialNodeIndex,
    parent: SpatialNodeIndex,
    expected_x: f32,
    expected_y: f32,
) {
    use euclid::approxeq::ApproxEq;
    const EPSILON: f32 = 0.0001;

    let p = LayoutPoint::new(px, py);
    let m = cst.get_relative_transform(child, parent).unwrap().flattened;
    let pt = m.transform_point2d(&p).unwrap();
    assert!(pt.x.approx_eq_eps(&expected_x, &EPSILON) &&
            pt.y.approx_eq_eps(&expected_y, &EPSILON),
            "p: {:?} -> {:?}\nm={:?}",
            p, pt, m,
            );
}

#[test]
fn test_cst_simple_translation() {
    // Basic translations only

    let mut cst = ClipScrollTree::new();

    let root = add_reference_frame(
        &mut cst,
        None,
        LayoutTransform::identity(),
        LayoutVector2D::zero(),
    );

    let child1 = add_reference_frame(
        &mut cst,
        Some(root),
        LayoutTransform::create_translation(100.0, 0.0, 0.0),
        LayoutVector2D::zero(),
    );

    let child2 = add_reference_frame(
        &mut cst,
        Some(child1),
        LayoutTransform::create_translation(0.0, 50.0, 0.0),
        LayoutVector2D::zero(),
    );

    let child3 = add_reference_frame(
        &mut cst,
        Some(child2),
        LayoutTransform::create_translation(200.0, 200.0, 0.0),
        LayoutVector2D::zero(),
    );

    cst.update_tree(WorldPoint::zero(), &SceneProperties::new(), None);

    test_pt(100.0, 100.0, &cst, child1, root, 200.0, 100.0);
    test_pt(100.0, 100.0, &cst, child2, root, 200.0, 150.0);
    test_pt(100.0, 100.0, &cst, child2, child1, 100.0, 150.0);
    test_pt(100.0, 100.0, &cst, child3, root, 400.0, 350.0);
}

#[test]
fn test_cst_simple_scale() {
    // Basic scale only

    let mut cst = ClipScrollTree::new();

    let root = add_reference_frame(
        &mut cst,
        None,
        LayoutTransform::identity(),
        LayoutVector2D::zero(),
    );

    let child1 = add_reference_frame(
        &mut cst,
        Some(root),
        LayoutTransform::create_scale(4.0, 1.0, 1.0),
        LayoutVector2D::zero(),
    );

    let child2 = add_reference_frame(
        &mut cst,
        Some(child1),
        LayoutTransform::create_scale(1.0, 2.0, 1.0),
        LayoutVector2D::zero(),
    );

    let child3 = add_reference_frame(
        &mut cst,
        Some(child2),
        LayoutTransform::create_scale(2.0, 2.0, 1.0),
        LayoutVector2D::zero(),
    );

    cst.update_tree(WorldPoint::zero(), &SceneProperties::new(), None);

    test_pt(100.0, 100.0, &cst, child1, root, 400.0, 100.0);
    test_pt(100.0, 100.0, &cst, child2, root, 400.0, 200.0);
    test_pt(100.0, 100.0, &cst, child3, root, 800.0, 400.0);
    test_pt(100.0, 100.0, &cst, child2, child1, 100.0, 200.0);
    test_pt(100.0, 100.0, &cst, child3, child1, 200.0, 400.0);
}

#[test]
fn test_cst_scale_translation() {
    // Scale + translation

    let mut cst = ClipScrollTree::new();

    let root = add_reference_frame(
        &mut cst,
        None,
        LayoutTransform::identity(),
        LayoutVector2D::zero(),
    );

    let child1 = add_reference_frame(
        &mut cst,
        Some(root),
        LayoutTransform::create_translation(100.0, 50.0, 0.0),
        LayoutVector2D::zero(),
    );

    let child2 = add_reference_frame(
        &mut cst,
        Some(child1),
        LayoutTransform::create_scale(2.0, 4.0, 1.0),
        LayoutVector2D::zero(),
    );

    let child3 = add_reference_frame(
        &mut cst,
        Some(child2),
        LayoutTransform::create_translation(200.0, -100.0, 0.0),
        LayoutVector2D::zero(),
    );

    let child4 = add_reference_frame(
        &mut cst,
        Some(child3),
        LayoutTransform::create_scale(3.0, 2.0, 1.0),
        LayoutVector2D::zero(),
    );

    cst.update_tree(WorldPoint::zero(), &SceneProperties::new(), None);

    test_pt(100.0, 100.0, &cst, child1, root, 200.0, 150.0);
    test_pt(100.0, 100.0, &cst, child2, root, 300.0, 450.0);
    test_pt(100.0, 100.0, &cst, child4, root, 1100.0, 450.0);

    test_pt(0.0, 0.0, &cst, child4, child1, 400.0, -400.0);
    test_pt(100.0, 100.0, &cst, child4, child1, 1000.0, 400.0);
    test_pt(100.0, 100.0, &cst, child2, child1, 200.0, 400.0);

    test_pt(100.0, 100.0, &cst, child3, child1, 600.0, 0.0);
}

#[test]
fn test_cst_translation_rotate() {
    // Rotation + translation
    use euclid::Angle;

    let mut cst = ClipScrollTree::new();

    let root = add_reference_frame(
        &mut cst,
        None,
        LayoutTransform::identity(),
        LayoutVector2D::zero(),
    );

    let child1 = add_reference_frame(
        &mut cst,
        Some(root),
        LayoutTransform::create_rotation(0.0, 0.0, 1.0, Angle::degrees(90.0)),
        LayoutVector2D::zero(),
    );

    cst.update_tree(WorldPoint::zero(), &SceneProperties::new(), None);

    test_pt(100.0, 0.0, &cst, child1, root, 0.0, -100.0);
}
