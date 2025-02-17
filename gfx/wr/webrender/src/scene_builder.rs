/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use api::{AsyncBlobImageRasterizer, BlobImageRequest, BlobImageParams, BlobImageResult};
use api::{DocumentId, PipelineId, ApiMsg, FrameMsg, ResourceUpdate, ExternalEvent, Epoch};
use api::{BuiltDisplayList, ColorF, LayoutSize, NotificationRequest, Checkpoint, IdNamespace};
use api::{MemoryReport};
use api::channel::MsgSender;
#[cfg(feature = "capture")]
use capture::CaptureConfig;
use frame_builder::{FrameBuilderConfig, FrameBuilder};
use clip_scroll_tree::ClipScrollTree;
use display_list_flattener::DisplayListFlattener;
use intern::{Internable, Interner};
use intern_types;
use internal_types::{FastHashMap, FastHashSet};
use malloc_size_of::{MallocSizeOf, MallocSizeOfOps};
use prim_store::{PrimitiveKeyKind};
use prim_store::PrimitiveStoreStats;
use prim_store::borders::{ImageBorder, NormalBorderPrim};
use prim_store::gradient::{LinearGradient, RadialGradient};
use prim_store::image::{Image, YuvImage};
use prim_store::line_dec::LineDecoration;
use prim_store::picture::Picture;
use prim_store::text_run::TextRun;
use resource_cache::{AsyncBlobImageInfo, FontInstanceMap};
use render_backend::DocumentView;
use renderer::{PipelineInfo, SceneBuilderHooks};
use scene::Scene;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::mem::replace;
use time::precise_time_ns;
use util::drain_filter;
use std::thread;
use std::time::Duration;

/// Represents the work associated to a transaction before scene building.
pub struct Transaction {
    pub document_id: DocumentId,
    pub display_list_updates: Vec<DisplayListUpdate>,
    pub removed_pipelines: Vec<PipelineId>,
    pub epoch_updates: Vec<(PipelineId, Epoch)>,
    pub request_scene_build: Option<SceneRequest>,
    pub blob_requests: Vec<BlobImageParams>,
    pub blob_rasterizer: Option<(Box<AsyncBlobImageRasterizer>, AsyncBlobImageInfo)>,
    pub rasterized_blobs: Vec<(BlobImageRequest, BlobImageResult)>,
    pub resource_updates: Vec<ResourceUpdate>,
    pub frame_ops: Vec<FrameMsg>,
    pub notifications: Vec<NotificationRequest>,
    pub set_root_pipeline: Option<PipelineId>,
    pub render_frame: bool,
    pub invalidate_rendered_frame: bool,
}

impl Transaction {
    pub fn can_skip_scene_builder(&self) -> bool {
        self.request_scene_build.is_none() &&
            self.display_list_updates.is_empty() &&
            self.epoch_updates.is_empty() &&
            self.removed_pipelines.is_empty() &&
            self.blob_requests.is_empty() &&
            self.set_root_pipeline.is_none()
    }

    pub fn should_build_scene(&self) -> bool {
        !self.display_list_updates.is_empty() ||
            self.set_root_pipeline.is_some()
    }

    fn rasterize_blobs(&mut self, is_low_priority: bool) {
        if let Some((ref mut rasterizer, _)) = self.blob_rasterizer {
            let mut rasterized_blobs = rasterizer.rasterize(&self.blob_requests, is_low_priority);
            // try using the existing allocation if our current list is empty
            if self.rasterized_blobs.is_empty() {
                self.rasterized_blobs = rasterized_blobs;
            } else {
                self.rasterized_blobs.append(&mut rasterized_blobs);
            }
        }
    }
}

/// Represent the remaining work associated to a transaction after the scene building
/// phase as well as the result of scene building itself if applicable.
pub struct BuiltTransaction {
    pub document_id: DocumentId,
    pub built_scene: Option<BuiltScene>,
    pub resource_updates: Vec<ResourceUpdate>,
    pub rasterized_blobs: Vec<(BlobImageRequest, BlobImageResult)>,
    pub blob_rasterizer: Option<(Box<AsyncBlobImageRasterizer>, AsyncBlobImageInfo)>,
    pub frame_ops: Vec<FrameMsg>,
    pub removed_pipelines: Vec<PipelineId>,
    pub notifications: Vec<NotificationRequest>,
    pub interner_updates: Option<InternerUpdates>,
    pub scene_build_start_time: u64,
    pub scene_build_end_time: u64,
    pub render_frame: bool,
    pub invalidate_rendered_frame: bool,
}

pub struct DisplayListUpdate {
    pub pipeline_id: PipelineId,
    pub epoch: Epoch,
    pub built_display_list: BuiltDisplayList,
    pub background: Option<ColorF>,
    pub viewport_size: LayoutSize,
    pub content_size: LayoutSize,
}

/// Contains the render backend data needed to build a scene.
pub struct SceneRequest {
    pub view: DocumentView,
    pub font_instances: FontInstanceMap,
    pub output_pipelines: FastHashSet<PipelineId>,
}

#[cfg(feature = "replay")]
pub struct LoadScene {
    pub document_id: DocumentId,
    pub scene: Scene,
    pub output_pipelines: FastHashSet<PipelineId>,
    pub font_instances: FontInstanceMap,
    pub view: DocumentView,
    pub config: FrameBuilderConfig,
    pub build_frame: bool,
    pub interners: Interners,
}

pub struct BuiltScene {
    pub scene: Scene,
    pub frame_builder: FrameBuilder,
    pub clip_scroll_tree: ClipScrollTree,
}

// Message from render backend to scene builder.
pub enum SceneBuilderRequest {
    Transaction(Box<Transaction>),
    ExternalEvent(ExternalEvent),
    DeleteDocument(DocumentId),
    WakeUp,
    Flush(MsgSender<()>),
    ClearNamespace(IdNamespace),
    SetFrameBuilderConfig(FrameBuilderConfig),
    SimulateLongSceneBuild(u32),
    SimulateLongLowPrioritySceneBuild(u32),
    Stop,
    ReportMemory(MemoryReport, MsgSender<MemoryReport>),
    #[cfg(feature = "capture")]
    SaveScene(CaptureConfig),
    #[cfg(feature = "replay")]
    LoadScenes(Vec<LoadScene>),
}

// Message from scene builder to render backend.
pub enum SceneBuilderResult {
    Transaction(Box<BuiltTransaction>, Option<Sender<SceneSwapResult>>),
    ExternalEvent(ExternalEvent),
    FlushComplete(MsgSender<()>),
    ClearNamespace(IdNamespace),
    Stopped,
}

// Message from render backend to scene builder to indicate the
// scene swap was completed. We need a separate channel for this
// so that they don't get mixed with SceneBuilderRequest messages.
pub enum SceneSwapResult {
    Complete(Sender<()>),
    Aborted,
}

macro_rules! declare_interners {
    ( $( $name: ident, )+ ) => {
        /// This struct contains all items that can be shared between
        /// display lists. We want to intern and share the same clips,
        /// primitives and other things between display lists so that:
        /// - GPU cache handles remain valid, reducing GPU cache updates.
        /// - Comparison of primitives and pictures between two
        ///   display lists is (a) fast (b) done during scene building.
        #[cfg_attr(feature = "capture", derive(Serialize))]
        #[cfg_attr(feature = "replay", derive(Deserialize))]
        #[derive(Default)]
        pub struct Interners {
            $(
                pub $name: intern_types::$name::Interner,
            )+
        }

        pub struct InternerUpdates {
            $(
                pub $name: intern_types::$name::UpdateList,
            )+
        }

        impl Interners {
            /// Reports CPU heap memory used by the interners.
            fn report_memory(
                &self,
                ops: &mut MallocSizeOfOps,
                r: &mut MemoryReport,
            ) {
                $(
                    r.interning.interners.$name += self.$name.size_of(ops);
                )+
            }

            fn end_frame_and_get_pending_updates(&mut self) -> InternerUpdates {
                InternerUpdates {
                    $(
                        $name: self.$name.end_frame_and_get_pending_updates(),
                    )+
                }
            }
        }
    }
}

enumerate_interners!(declare_interners);

// Access to `Interners` interners by `Internable`
pub trait InternerMut<I: Internable>
{
    fn interner_mut(&mut self) -> &mut Interner<I::Source, I::InternData, I::Marker>;
}

macro_rules! impl_interner_mut {
    ($($ty:ident: $mem:ident,)*) => {
        $(impl InternerMut<$ty> for Interners {
            fn interner_mut(&mut self) -> &mut Interner<
                <$ty as Internable>::Source,
                <$ty as Internable>::InternData,
                <$ty as Internable>::Marker
            > {
                &mut self.$mem
            }
        })*
    }
}

impl_interner_mut! {
    Image: image,
    ImageBorder: image_border,
    LineDecoration: line_decoration,
    LinearGradient: linear_grad,
    NormalBorderPrim: normal_border,
    Picture: picture,
    PrimitiveKeyKind: prim,
    RadialGradient: radial_grad,
    TextRun: text_run,
    YuvImage: yuv_image,
}

// A document in the scene builder contains the current scene,
// as well as a persistent clip interner. This allows clips
// to be de-duplicated, and persisted in the GPU cache between
// display lists.
struct Document {
    scene: Scene,
    interners: Interners,
    prim_store_stats: PrimitiveStoreStats,
}

impl Document {
    fn new(scene: Scene) -> Self {
        Document {
            scene,
            interners: Interners::default(),
            prim_store_stats: PrimitiveStoreStats::empty(),
        }
    }
}

pub struct SceneBuilder {
    documents: FastHashMap<DocumentId, Document>,
    rx: Receiver<SceneBuilderRequest>,
    tx: Sender<SceneBuilderResult>,
    api_tx: MsgSender<ApiMsg>,
    config: FrameBuilderConfig,
    hooks: Option<Box<SceneBuilderHooks + Send>>,
    simulate_slow_ms: u32,
    size_of_ops: Option<MallocSizeOfOps>,
}

impl SceneBuilder {
    pub fn new(
        config: FrameBuilderConfig,
        api_tx: MsgSender<ApiMsg>,
        hooks: Option<Box<SceneBuilderHooks + Send>>,
        size_of_ops: Option<MallocSizeOfOps>,
    ) -> (Self, Sender<SceneBuilderRequest>, Receiver<SceneBuilderResult>) {
        let (in_tx, in_rx) = channel();
        let (out_tx, out_rx) = channel();
        (
            SceneBuilder {
                documents: FastHashMap::default(),
                rx: in_rx,
                tx: out_tx,
                api_tx,
                config,
                hooks,
                size_of_ops,
                simulate_slow_ms: 0,
            },
            in_tx,
            out_rx,
        )
    }

    /// Send a message to the render backend thread.
    ///
    /// We first put something in the result queue and then send a wake-up
    /// message to the api queue that the render backend is blocking on.
    pub fn send(&self, msg: SceneBuilderResult) {
        self.tx.send(msg).unwrap();
        let _ = self.api_tx.send(ApiMsg::WakeUp);
    }

    /// The scene builder thread's event loop.
    pub fn run(&mut self) {
        if let Some(ref hooks) = self.hooks {
            hooks.register();
        }

        loop {
            match self.rx.recv() {
                Ok(SceneBuilderRequest::WakeUp) => {}
                Ok(SceneBuilderRequest::Flush(tx)) => {
                    self.send(SceneBuilderResult::FlushComplete(tx));
                }
                Ok(SceneBuilderRequest::Transaction(mut txn)) => {
                    let built_txn = self.process_transaction(&mut txn);
                    self.forward_built_transaction(built_txn);
                }
                Ok(SceneBuilderRequest::DeleteDocument(document_id)) => {
                    self.documents.remove(&document_id);
                }
                Ok(SceneBuilderRequest::SetFrameBuilderConfig(cfg)) => {
                    self.config = cfg;
                }
                Ok(SceneBuilderRequest::ClearNamespace(id)) => {
                    self.documents.retain(|doc_id, _doc| doc_id.0 != id);
                    self.send(SceneBuilderResult::ClearNamespace(id));
                }
                #[cfg(feature = "replay")]
                Ok(SceneBuilderRequest::LoadScenes(msg)) => {
                    self.load_scenes(msg);
                }
                #[cfg(feature = "capture")]
                Ok(SceneBuilderRequest::SaveScene(config)) => {
                    self.save_scene(config);
                }
                Ok(SceneBuilderRequest::ExternalEvent(evt)) => {
                    self.send(SceneBuilderResult::ExternalEvent(evt));
                }
                Ok(SceneBuilderRequest::Stop) => {
                    self.tx.send(SceneBuilderResult::Stopped).unwrap();
                    // We don't need to send a WakeUp to api_tx because we only
                    // get the Stop when the RenderBackend loop is exiting.
                    break;
                }
                Ok(SceneBuilderRequest::ReportMemory(mut report, tx)) => {
                    report += self.report_memory();
                    tx.send(report).unwrap();
                }
                Ok(SceneBuilderRequest::SimulateLongSceneBuild(time_ms)) => {
                    self.simulate_slow_ms = time_ms
                }
                Ok(SceneBuilderRequest::SimulateLongLowPrioritySceneBuild(_)) => {}
                Err(_) => {
                    break;
                }
            }

            if let Some(ref hooks) = self.hooks {
                hooks.poke();
            }
        }

        if let Some(ref hooks) = self.hooks {
            hooks.deregister();
        }
    }

    #[cfg(feature = "capture")]
    fn save_scene(&mut self, config: CaptureConfig) {
        for (id, doc) in &self.documents {
            let interners_name = format!("interners-{}-{}", (id.0).0, id.1);
            config.serialize(&doc.interners, interners_name);
        }
    }

    #[cfg(feature = "replay")]
    fn load_scenes(&mut self, scenes: Vec<LoadScene>) {
        for mut item in scenes {
            self.config = item.config;

            let scene_build_start_time = precise_time_ns();

            let mut built_scene = None;
            let mut interner_updates = None;

            if item.scene.has_root_pipeline() {
                let mut clip_scroll_tree = ClipScrollTree::new();
                let mut new_scene = Scene::new();

                let frame_builder = DisplayListFlattener::create_frame_builder(
                    &item.scene,
                    &mut clip_scroll_tree,
                    item.font_instances,
                    &item.view,
                    &item.output_pipelines,
                    &self.config,
                    &mut new_scene,
                    &mut item.interners,
                    &PrimitiveStoreStats::empty(),
                );

                interner_updates = Some(
                    item.interners.end_frame_and_get_pending_updates()
                );

                built_scene = Some(BuiltScene {
                    scene: new_scene,
                    frame_builder,
                    clip_scroll_tree,
                });
            }

            self.documents.insert(
                item.document_id,
                Document {
                    scene: item.scene,
                    interners: item.interners,
                    prim_store_stats: PrimitiveStoreStats::empty(),
                },
            );

            let txn = Box::new(BuiltTransaction {
                document_id: item.document_id,
                render_frame: item.build_frame,
                invalidate_rendered_frame: false,
                built_scene,
                resource_updates: Vec::new(),
                rasterized_blobs: Vec::new(),
                blob_rasterizer: None,
                frame_ops: Vec::new(),
                removed_pipelines: Vec::new(),
                notifications: Vec::new(),
                scene_build_start_time,
                scene_build_end_time: precise_time_ns(),
                interner_updates,
            });

            self.forward_built_transaction(txn);
        }
    }

    /// Do the bulk of the work of the scene builder thread.
    fn process_transaction(&mut self, txn: &mut Transaction) -> Box<BuiltTransaction> {
        if let &Some(ref hooks) = &self.hooks {
            hooks.pre_scene_build();
        }

        let scene_build_start_time = precise_time_ns();

        let doc = self.documents
                      .entry(txn.document_id)
                      .or_insert(Document::new(Scene::new()));
        let scene = &mut doc.scene;

        for update in txn.display_list_updates.drain(..) {
            scene.set_display_list(
                update.pipeline_id,
                update.epoch,
                update.built_display_list,
                update.background,
                update.viewport_size,
                update.content_size,
            );
        }

        for &(pipeline_id, epoch) in &txn.epoch_updates {
            scene.update_epoch(pipeline_id, epoch);
        }

        if let Some(id) = txn.set_root_pipeline {
            scene.set_root_pipeline_id(id);
        }

        for pipeline_id in &txn.removed_pipelines {
            scene.remove_pipeline(*pipeline_id)
        }

        let mut built_scene = None;
        let mut interner_updates = None;
        if scene.has_root_pipeline() {
            if let Some(request) = txn.request_scene_build.take() {
                let mut clip_scroll_tree = ClipScrollTree::new();
                let mut new_scene = Scene::new();

                let frame_builder = DisplayListFlattener::create_frame_builder(
                    &scene,
                    &mut clip_scroll_tree,
                    request.font_instances,
                    &request.view,
                    &request.output_pipelines,
                    &self.config,
                    &mut new_scene,
                    &mut doc.interners,
                    &doc.prim_store_stats,
                );

                // Update the allocation stats for next scene
                doc.prim_store_stats = frame_builder.prim_store.get_stats();

                // Retrieve the list of updates from the clip interner.
                interner_updates = Some(
                    doc.interners.end_frame_and_get_pending_updates()
                );

                built_scene = Some(BuiltScene {
                    scene: new_scene,
                    frame_builder,
                    clip_scroll_tree,
                });
            }
        }

        let is_low_priority = false;
        txn.rasterize_blobs(is_low_priority);

        drain_filter(
            &mut txn.notifications,
            |n| { n.when() == Checkpoint::SceneBuilt },
            |n| { n.notify(); },
        );

        if self.simulate_slow_ms > 0 {
            thread::sleep(Duration::from_millis(self.simulate_slow_ms as u64));
        }

        Box::new(BuiltTransaction {
            document_id: txn.document_id,
            render_frame: txn.render_frame,
            invalidate_rendered_frame: txn.invalidate_rendered_frame,
            built_scene,
            rasterized_blobs: replace(&mut txn.rasterized_blobs, Vec::new()),
            resource_updates: replace(&mut txn.resource_updates, Vec::new()),
            blob_rasterizer: replace(&mut txn.blob_rasterizer, None),
            frame_ops: replace(&mut txn.frame_ops, Vec::new()),
            removed_pipelines: replace(&mut txn.removed_pipelines, Vec::new()),
            notifications: replace(&mut txn.notifications, Vec::new()),
            interner_updates,
            scene_build_start_time,
            scene_build_end_time: precise_time_ns(),
        })
    }

    /// Send the result of process_transaction back to the render backend.
    fn forward_built_transaction(&mut self, txn: Box<BuiltTransaction>) {
        // We only need the pipeline info and the result channel if we
        // have a hook callback *and* if this transaction actually built
        // a new scene that is going to get swapped in. In other cases
        // pipeline_info can be None and we can avoid some overhead from
        // invoking the hooks and blocking on the channel.
        let (pipeline_info, result_tx, result_rx) = match (&self.hooks, &txn.built_scene) {
            (&Some(ref hooks), &Some(ref built)) => {
                let info = PipelineInfo {
                    epochs: built.scene.pipeline_epochs.clone(),
                    removed_pipelines: txn.removed_pipelines.clone(),
                };
                let (tx, rx) = channel();

                hooks.pre_scene_swap(txn.scene_build_end_time - txn.scene_build_start_time);

                (Some(info), Some(tx), Some(rx))
            }
            _ => (None, None, None),
        };

        let scene_swap_start_time = precise_time_ns();
        let has_resources_updates = !txn.resource_updates.is_empty();
        let invalidate_rendered_frame = txn.invalidate_rendered_frame;

        self.tx.send(SceneBuilderResult::Transaction(txn, result_tx)).unwrap();

        let _ = self.api_tx.send(ApiMsg::WakeUp);

        if let Some(pipeline_info) = pipeline_info {
            // Block until the swap is done, then invoke the hook.
            let swap_result = result_rx.unwrap().recv();
            let scene_swap_time = precise_time_ns() - scene_swap_start_time;
            self.hooks.as_ref().unwrap().post_scene_swap(pipeline_info, scene_swap_time);
            // Once the hook is done, allow the RB thread to resume
            match swap_result {
                Ok(SceneSwapResult::Complete(resume_tx)) => {
                    resume_tx.send(()).ok();
                },
                _ => (),
            };
        } else if has_resources_updates || invalidate_rendered_frame {
            if let &Some(ref hooks) = &self.hooks {
                hooks.post_resource_update();
            }
        } else {
            if let &Some(ref hooks) = &self.hooks {
                hooks.post_empty_scene_build();
            }
        }
    }

    /// Reports CPU heap memory used by the SceneBuilder.
    fn report_memory(&mut self) -> MemoryReport {
        let ops = self.size_of_ops.as_mut().unwrap();
        let mut report = MemoryReport::default();
        for doc in self.documents.values() {
            doc.interners.report_memory(ops, &mut report);
        }

        report
    }
}

/// A scene builder thread which executes expensive operations such as blob rasterization
/// with a lower priority than the normal scene builder thread.
///
/// After rasterizing blobs, the secene building request is forwarded to the normal scene
/// builder where the FrameBuilder is generated.
pub struct LowPrioritySceneBuilder {
    pub rx: Receiver<SceneBuilderRequest>,
    pub tx: Sender<SceneBuilderRequest>,
    pub simulate_slow_ms: u32,
}

impl LowPrioritySceneBuilder {
    pub fn run(&mut self) {
        loop {
            match self.rx.recv() {
                Ok(SceneBuilderRequest::Transaction(txn)) => {
                    let txn = self.process_transaction(txn);
                    self.tx.send(SceneBuilderRequest::Transaction(txn)).unwrap();
                }
                Ok(SceneBuilderRequest::DeleteDocument(document_id)) => {
                    self.tx.send(SceneBuilderRequest::DeleteDocument(document_id)).unwrap();
                }
                Ok(SceneBuilderRequest::Stop) => {
                    self.tx.send(SceneBuilderRequest::Stop).unwrap();
                    break;
                }
                Ok(SceneBuilderRequest::SimulateLongLowPrioritySceneBuild(time_ms)) => {
                    self.simulate_slow_ms = time_ms;
                }
                Ok(other) => {
                    self.tx.send(other).unwrap();
                }
                Err(_) => {
                    break;
                }
            }
        }
    }

    fn process_transaction(&mut self, mut txn: Box<Transaction>) -> Box<Transaction> {
        let is_low_priority = true;
        txn.rasterize_blobs(is_low_priority);
        txn.blob_requests = Vec::new();

        if self.simulate_slow_ms > 0 {
            thread::sleep(Duration::from_millis(self.simulate_slow_ms as u64));
        }

        txn
    }
}
