#![allow(deprecated)]

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("This example is only supported on macos")
}

#[cfg(all(target_os = "macos", not(feature = "metal")))]
fn main() {
    println!("To run this example, invoke cargo with --features \"metal\".")
}

#[cfg(all(target_os = "macos", feature = "metal"))]
fn main() {
    use objc2::{
        rc::{autoreleasepool, Retained},
        runtime::ProtocolObject,
    };
    use objc2_core_foundation::CGSize;
    use objc2_metal::{MTLCommandBuffer, MTLCommandQueue};
    use objc2_quartz_core::CAMetalDrawable;

    use winit::{
        application::ApplicationHandler,
        event::WindowEvent,
        event_loop::{ActiveEventLoop, EventLoop},
        window::WindowId,
    };

    use skia_safe::{
        gpu::{self, backend_render_targets, mtl, SurfaceOrigin},
        scalar, ColorType,
    };

    let event_loop = EventLoop::new().expect("Failed to create event loop");

    struct Application {
        context: Option<window::Context>,
    }

    let mut application = Application { context: None };

    impl ApplicationHandler for Application {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            assert!(self.context.is_none());
            self.context = Some(window::Context::new(event_loop))
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            event: WindowEvent,
        ) {
            let context = &mut self.context.as_mut().unwrap();
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(size) => {
                    context
                        .metal_layer
                        .setDrawableSize(CGSize::new(size.width as f64, size.height as f64));
                    context.window.request_redraw()
                }
                WindowEvent::RedrawRequested => {
                    if let Some(drawable) = context.metal_layer.nextDrawable() {
                        let (drawable_width, drawable_height) = {
                            let size = context.metal_layer.drawableSize();
                            (size.width as scalar, size.height as scalar)
                        };

                        let mut surface = {
                            let texture_info = unsafe {
                                mtl::TextureInfo::new(
                                    Retained::as_ptr(&drawable.texture()) as mtl::Handle
                                )
                            };

                            let backend_render_target = backend_render_targets::make_mtl(
                                (drawable_width as i32, drawable_height as i32),
                                &texture_info,
                            );

                            gpu::surfaces::wrap_backend_render_target(
                                &mut context.skia,
                                &backend_render_target,
                                SurfaceOrigin::TopLeft,
                                ColorType::BGRA8888,
                                None,
                                None,
                            )
                            .unwrap()
                        };

                        window::draw(surface.canvas());

                        context.skia.flush_and_submit();
                        drop(surface);

                        let command_buffer = context
                            .command_queue
                            .commandBuffer()
                            .expect("unable to get command buffer");

                        let drawable: Retained<ProtocolObject<dyn objc2_metal::MTLDrawable>> =
                            (&drawable).into();
                        command_buffer.presentDrawable(&drawable);
                        command_buffer.commit();
                    }
                }
                _ => (),
            }
        }
    }

    // As of winit 0.30.0, this is crashing on exit:
    // https://github.com/rust-windowing/winit/issues/3668
    autoreleasepool(|_| {
        event_loop.run_app(&mut application).expect("run() failed");
    })
}

#[cfg(all(target_os = "macos", feature = "metal"))]
mod window {
    use objc2::{rc::Retained, runtime::ProtocolObject};
    use objc2_core_foundation::CGSize;
    use objc2_metal::{MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice};
    use objc2_quartz_core::CAMetalLayer;

    #[cfg(target_os = "macos")]
    use objc2_app_kit::NSView;

    #[cfg(target_os = "ios")]
    use objc2_ui_kit::UIView;

    use skia_safe::{
        gpu::{self, mtl, DirectContext},
        Canvas, Color4f, Paint, Point, Rect,
    };
    use winit::{
        dpi::{LogicalSize, Size},
        event_loop::ActiveEventLoop,
        raw_window_handle::HasWindowHandle,
        window::{Window, WindowAttributes},
    };

    pub struct Context {
        pub window: Window,
        pub metal_layer: Retained<CAMetalLayer>,
        pub command_queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
        pub skia: DirectContext,
    }

    impl Context {
        pub fn new(event_loop: &ActiveEventLoop) -> Self {
            let size = LogicalSize::new(800, 600);
            let mut window_attributes = WindowAttributes::default();
            window_attributes.inner_size = Some(Size::new(size));
            window_attributes.title = "Skia Metal Winit Example".to_string();

            let window = event_loop
                .create_window(window_attributes)
                .expect("Failed to create Window");

            let device = MTLCreateSystemDefaultDevice().expect("no device found");

            let metal_layer = {
                let layer = CAMetalLayer::new();
                layer.setDevice(Some(&device));
                layer.setPixelFormat(objc2_metal::MTLPixelFormat::BGRA8Unorm);
                layer.setPresentsWithTransaction(false);
                // Disabling this option allows Skia's Blend Mode to work.
                // More about: https://developer.apple.com/documentation/quartzcore/cametallayer/1478168-framebufferonly
                layer.setFramebufferOnly(false);
                layer.setDrawableSize(CGSize::new(size.width as f64, size.height as f64));

                let view_ptr = match window.window_handle().unwrap().as_raw() {
                    #[cfg(target_os = "macos")]
                    raw_window_handle::RawWindowHandle::AppKit(appkit) => {
                        appkit.ns_view.as_ptr() as *mut NSView
                    }
                    #[cfg(target_os = "ios")]
                    raw_window_handle::RawWindowHandle::UiKit(uikit) => {
                        uikit.ui_view.as_ptr() as *mut UIView
                    }
                    _ => panic!("Wrong window handle type"),
                };
                let view = unsafe { view_ptr.as_ref().unwrap() };

                #[cfg(target_os = "macos")]
                {
                    view.setWantsLayer(true);
                    view.setLayer(Some(&layer.clone().into_super()));
                }

                #[cfg(target_os = "ios")]
                {
                    // TODO: consider using raw-window-metal crate. It synchronises some properties
                    // from the parent UIView layer to the child metal layer when they change
                    layer.setFrame(view.layer().frame());
                    view.layer().addSublayer(&layer)
                }

                layer
            };

            let command_queue = device
                .newCommandQueue()
                .expect("unable to get command queue");

            let backend = unsafe {
                mtl::BackendContext::new(
                    Retained::as_ptr(&device) as mtl::Handle,
                    Retained::as_ptr(&command_queue) as mtl::Handle,
                )
            };

            let skia_context = gpu::direct_contexts::make_metal(&backend, None).unwrap();

            Self {
                window,
                metal_layer,
                command_queue,
                skia: skia_context,
            }
        }
    }

    /// Renders a rectangle that occupies exactly half of the canvas
    pub fn draw(canvas: &Canvas) {
        let canvas_size = skia_safe::Size::from(canvas.base_layer_size());

        canvas.clear(Color4f::new(1.0, 1.0, 1.0, 1.0));

        let rect_size = canvas_size / 2.0;
        let rect = Rect::from_point_and_size(
            Point::new(
                (canvas_size.width - rect_size.width) / 2.0,
                (canvas_size.height - rect_size.height) / 2.0,
            ),
            rect_size,
        );
        canvas.draw_rect(rect, &Paint::new(Color4f::new(0.0, 0.0, 1.0, 1.0), None));
    }
}
