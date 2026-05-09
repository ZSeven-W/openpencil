#[cfg(not(feature = "vulkan"))]
fn main() {
    println!("To run this example, invoke cargo with --features \"vulkan\".")
}

#[cfg(feature = "vulkan")]
mod context;

#[cfg(feature = "vulkan")]
mod renderer;

#[cfg(feature = "vulkan")]
fn main() {
    use skia_safe::{Color4f, Paint, Point, Rect};
    use std::sync::Arc;
    use winit::{
        application::ApplicationHandler,
        event::WindowEvent,
        event_loop::{ActiveEventLoop, EventLoop},
        window::{Window, WindowId},
    };

    use context::VulkanRenderContext;
    use renderer::VulkanRenderer;

    #[derive(Default)]
    struct App {
        render_ctx: VulkanRenderContext, // the shared vulkan device, queue, etc.
        renderer: Option<VulkanRenderer>, // the window-specific skia <-> vulkan bridge
    }

    impl ApplicationHandler for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            // since the renderer needs to hold onto a reference to the window, we wrap it in an Arc
            let window = Arc::new(
                event_loop
                    .create_window(Window::default_attributes())
                    .unwrap(),
            );

            // in this example we only have a single window, but you could also keep a list of
            // VulkanRenderer instances to manage multiple windows
            self.renderer = Some(
                self.render_ctx
                    .renderer_for_window(event_loop, window.clone()),
            );
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            event: WindowEvent,
        ) {
            match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                WindowEvent::Resized(_) => {
                    if let Some(renderer) = self.renderer.as_mut() {
                        // When the window size changes, swapchain-dependent resources need to be
                        // recreated before redrawing the window contents.
                        renderer.invalidate_swapchain();
                        renderer.window.request_redraw();
                    }
                }
                WindowEvent::RedrawRequested => {
                    if let Some(renderer) = self.renderer.as_mut() {
                        // The swapchain (which manages presentable images and update timing) needs
                        // to be cleaned up/validated in between redraws.
                        renderer.prepare_swapchain();

                        // After the draw routine completes, the contents of the canvas will be displayed
                        renderer.draw_and_present(|canvas, size| {
                            let canvas_size = skia_safe::Size::new(size.width, size.height);
                            canvas.clear(Color4f::new(1.0, 1.0, 1.0, 1.0));

                            let rect_size = canvas_size / 2.0;
                            let rect = Rect::from_point_and_size(
                                Point::new(
                                    (canvas_size.width - rect_size.width) / 2.0,
                                    (canvas_size.height - rect_size.height) / 2.0,
                                ),
                                rect_size,
                            );
                            canvas.draw_rect(
                                rect,
                                &Paint::new(Color4f::new(0.0, 0.0, 1.0, 1.0), None),
                            );
                        });
                    }
                }
                _ => {}
            }
        }
    }

    let event_loop = EventLoop::new().unwrap();
    let validate = std::env::args().any(|arg| arg == "--validate");
    if validate {
        println!("Vulkan validation requested via --validate");
    }

    let mut app = App {
        render_ctx: VulkanRenderContext::new(validate),
        renderer: None,
    };

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    event_loop.run_app(&mut app).ok();
}
