/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate euclid;
extern crate gleam;
extern crate glutin;
extern crate webrender;
extern crate winit;

#[path = "common/boilerplate.rs"]
mod boilerplate;

use boilerplate::{Example, HandyDandyRectBuilder};
use euclid::SideOffsets2D;
use webrender::api::*;
use winit::dpi::LogicalPosition;

struct App {
    cursor_position: WorldPoint,
}

impl Example for App {
    fn render(
        &mut self,
        _api: &RenderApi,
        builder: &mut DisplayListBuilder,
        _txn: &mut Transaction,
        _framebuffer_size: DeviceIntSize,
        pipeline_id: PipelineId,
        _document_id: DocumentId,
    ) {
        let info = LayoutPrimitiveInfo::new(
            LayoutRect::new(LayoutPoint::zero(), builder.content_size())
        );
        let root_space_and_clip = SpaceAndClipInfo::root_scroll(pipeline_id);
        builder.push_simple_stacking_context(
            &info,
            root_space_and_clip.spatial_id,
        );

        if true {
            // scrolling and clips stuff
            // let's make a scrollbox
            let scrollbox = (0, 0).to(300, 400);
            builder.push_simple_stacking_context(
                &LayoutPrimitiveInfo::new((10, 10).by(0, 0)),
                root_space_and_clip.spatial_id,
            );
            // set the scrolling clip
            let space_and_clip1 = builder.define_scroll_frame(
                &root_space_and_clip,
                None,
                (0, 0).by(1000, 1000),
                scrollbox,
                vec![],
                None,
                ScrollSensitivity::ScriptAndInputEvents,
                LayoutPoint::zero(),
            );

            // now put some content into it.
            // start with a white background
            let mut info = LayoutPrimitiveInfo::new((0, 0).to(1000, 1000));
            info.tag = Some((0, 1));
            builder.push_rect(&info, &space_and_clip1, ColorF::new(1.0, 1.0, 1.0, 1.0));

            // let's make a 50x50 blue square as a visual reference
            let mut info = LayoutPrimitiveInfo::new((0, 0).to(50, 50));
            info.tag = Some((0, 2));
            builder.push_rect(&info, &space_and_clip1, ColorF::new(0.0, 0.0, 1.0, 1.0));

            // and a 50x50 green square next to it with an offset clip
            // to see what that looks like
            let mut info =
                LayoutPrimitiveInfo::with_clip_rect((50, 0).to(100, 50), (60, 10).to(110, 60));
            info.tag = Some((0, 3));
            builder.push_rect(&info, &space_and_clip1, ColorF::new(0.0, 1.0, 0.0, 1.0));

            // Below the above rectangles, set up a nested scrollbox. It's still in
            // the same stacking context, so note that the rects passed in need to
            // be relative to the stacking context.
            let space_and_clip2 = builder.define_scroll_frame(
                &space_and_clip1,
                None,
                (0, 100).to(300, 1000),
                (0, 100).to(200, 300),
                vec![],
                None,
                ScrollSensitivity::ScriptAndInputEvents,
                LayoutPoint::zero(),
            );

            // give it a giant gray background just to distinguish it and to easily
            // visually identify the nested scrollbox
            let mut info = LayoutPrimitiveInfo::new((-1000, -1000).to(5000, 5000));
            info.tag = Some((0, 4));
            builder.push_rect(&info, &space_and_clip2, ColorF::new(0.5, 0.5, 0.5, 1.0));

            // add a teal square to visualize the scrolling/clipping behaviour
            // as you scroll the nested scrollbox
            let mut info = LayoutPrimitiveInfo::new((0, 200).to(50, 250));
            info.tag = Some((0, 5));
            builder.push_rect(&info, &space_and_clip2, ColorF::new(0.0, 1.0, 1.0, 1.0));

            // Add a sticky frame. It will "stick" twice while scrolling, once
            // at a margin of 10px from the bottom, for 40 pixels of scrolling,
            // and once at a margin of 10px from the top, for 60 pixels of
            // scrolling.
            let sticky_id = builder.define_sticky_frame(
                space_and_clip2.spatial_id,
                (50, 350).by(50, 50),
                SideOffsets2D::new(Some(10.0), None, Some(10.0), None),
                StickyOffsetBounds::new(-40.0, 60.0),
                StickyOffsetBounds::new(0.0, 0.0),
                LayoutVector2D::new(0.0, 0.0)
            );

            let mut info = LayoutPrimitiveInfo::new((50, 350).by(50, 50));
            info.tag = Some((0, 6));
            builder.push_rect(
                &info,
                &SpaceAndClipInfo {
                    spatial_id: sticky_id,
                    clip_id: space_and_clip2.clip_id,
                },
                ColorF::new(0.5, 0.5, 1.0, 1.0),
            );

            // just for good measure add another teal square further down and to
            // the right, which can be scrolled into view by the user
            let mut info = LayoutPrimitiveInfo::new((250, 350).to(300, 400));
            info.tag = Some((0, 7));
            builder.push_rect(&info, &space_and_clip2, ColorF::new(0.0, 1.0, 1.0, 1.0));

            builder.pop_stacking_context();
        }

        builder.pop_stacking_context();
    }

    fn on_event(&mut self, event: winit::WindowEvent, api: &RenderApi, document_id: DocumentId) -> bool {
        let mut txn = Transaction::new();
        match event {
            winit::WindowEvent::KeyboardInput {
                input: winit::KeyboardInput {
                    state: winit::ElementState::Pressed,
                    virtual_keycode: Some(key),
                    ..
                },
                ..
            } => {
                let offset = match key {
                    winit::VirtualKeyCode::Down => Some((0.0, -10.0)),
                    winit::VirtualKeyCode::Up => Some((0.0, 10.0)),
                    winit::VirtualKeyCode::Right => Some((-10.0, 0.0)),
                    winit::VirtualKeyCode::Left => Some((10.0, 0.0)),
                    _ => None,
                };
                let zoom = match key {
                    winit::VirtualKeyCode::Key0 => Some(1.0),
                    winit::VirtualKeyCode::Minus => Some(0.8),
                    winit::VirtualKeyCode::Equals => Some(1.25),
                    _ => None,
                };

                if let Some(offset) = offset {
                    txn.scroll(
                        ScrollLocation::Delta(LayoutVector2D::new(offset.0, offset.1)),
                        self.cursor_position,
                    );
                    txn.generate_frame();
                }
                if let Some(zoom) = zoom {
                    txn.set_pinch_zoom(ZoomFactor::new(zoom));
                    txn.generate_frame();
                }
            }
            winit::WindowEvent::CursorMoved { position: LogicalPosition { x, y }, .. } => {
                self.cursor_position = WorldPoint::new(x as f32, y as f32);
            }
            winit::WindowEvent::MouseWheel { delta, .. } => {
                const LINE_HEIGHT: f32 = 38.0;
                let (dx, dy) = match delta {
                    winit::MouseScrollDelta::LineDelta(dx, dy) => (dx, dy * LINE_HEIGHT),
                    winit::MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
                };

                txn.scroll(
                    ScrollLocation::Delta(LayoutVector2D::new(dx, dy)),
                    self.cursor_position,
                );
                txn.generate_frame();
            }
            winit::WindowEvent::MouseInput { .. } => {
                let results = api.hit_test(
                    document_id,
                    None,
                    self.cursor_position,
                    HitTestFlags::FIND_ALL
                );

                println!("Hit test results:");
                for item in &results.items {
                    println!("  • {:?}", item);
                }
                println!("");
            }
            _ => (),
        }

        api.send_transaction(document_id, txn);

        false
    }
}

fn main() {
    let mut app = App {
        cursor_position: WorldPoint::zero(),
    };
    boilerplate::main_wrapper(&mut app, None);
}
