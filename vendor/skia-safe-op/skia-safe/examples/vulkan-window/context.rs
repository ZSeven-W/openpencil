use std::sync::Arc;
use vulkano::{
    device::{
        physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, Queue,
        QueueCreateInfo, QueueFlags,
    },
    instance::debug::{
        DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessengerCallback,
        DebugUtilsMessengerCreateInfo,
    },
    instance::{Instance, InstanceCreateFlags, InstanceCreateInfo, InstanceExtensions},
    swapchain::Surface,
    VulkanLibrary,
};

use winit::{event_loop::ActiveEventLoop, window::Window};

use super::renderer::VulkanRenderer;

#[derive(Default)]
pub struct VulkanRenderContext {
    pub queue: Option<Arc<Queue>>,
    pub validate: bool,
}

impl VulkanRenderContext {
    pub fn new(validate: bool) -> Self {
        Self {
            queue: None,
            validate,
        }
    }

    pub fn renderer_for_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: Arc<Window>,
    ) -> VulkanRenderer {
        // lazily set up a shared instance, device, and queue to use for all subsequent renderers
        let validate = self.validate;
        let queue = self
            .queue
            .get_or_insert_with(|| Self::shared_queue(event_loop, window.clone(), validate));

        VulkanRenderer::new(window.clone(), queue.clone())
    }

    fn shared_queue(
        event_loop: &ActiveEventLoop,
        window: Arc<Window>,
        validate: bool,
    ) -> Arc<Queue> {
        let library = VulkanLibrary::new().expect("Vulkan libraries not found on system");

        // The first step of any Vulkan program is to create an instance.
        //
        // When we create an instance, we have to pass a list of extensions that we want to enable.
        //
        // All the window-drawing functionalities are part of non-core extensions that we need to
        // enable manually. To do so, we ask `Surface` for the list of extensions required to draw
        // to a window.
        let required_extensions = Surface::required_extensions(event_loop).unwrap();
        let (enabled_extensions, enabled_layers, debug_utils_messengers) =
            Self::validation_instance_config(&library, required_extensions, validate);

        // Now creating the instance.
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                // Enable enumerating devices that use non-conformant Vulkan implementations.
                // (e.g. MoltenVK)
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_extensions,
                enabled_layers,
                debug_utils_messengers,
                ..Default::default()
            },
        )
        .unwrap_or_else(|_| {
            panic!("Could not create instance supporting: {required_extensions:?}")
        });

        // Choose device extensions that we're going to use. In order to present images to a
        // surface, we need a `Swapchain`, which is provided by the `khr_swapchain` extension.
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        // In order to select the proper queue family we need a reference to the window's surface
        // so we can check whether the queue supports it. Note that in a future vulkano release
        // this requirement will go away once it can check for `presentation_support` from the
        // event_loop's display (see commented usage below…)
        let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();

        // We then choose which physical device to use. First, we enumerate all the available
        // physical devices, then apply filters to narrow them down to those that can support our
        // needs.
        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .unwrap()
            .filter(|p| {
                // Some devices may not support the extensions or features that your application,
                // or report properties and limits that are not sufficient for your application.
                // These should be filtered out here.
                p.supported_extensions().contains(&device_extensions)
            })
            .filter_map(|p| {
                // For each physical device, we try to find a suitable queue family that will
                // execute our draw commands.
                //
                // Devices can provide multiple queues to run commands in parallel (for example a
                // draw queue and a compute queue), similar to CPU threads. This is
                // something you have to have to manage manually in Vulkan. Queues
                // of the same type belong to the same queue family.
                //
                // Here, we look for a single queue family that is suitable for our purposes. In a
                // real-world application, you may want to use a separate dedicated transfer queue
                // to handle data transfers in parallel with graphics operations.
                // You may also need a separate queue for compute operations, if
                // your application uses those.
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        // We select a queue family that supports graphics operations. When drawing
                        // to a window surface, as we do in this example, we also need to check
                        // that queues in this queue family are capable of presenting images to the
                        // surface.
                        q.queue_flags.intersects(QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                        //  && p.presentation_support(_i as u32, event_loop).unwrap() // unreleased
                    })
                    // The code here searches for the first queue family that is suitable. If none
                    // is found, `None` is returned to `filter_map`, which
                    // disqualifies this physical device.
                    .map(|i| (p, i as u32))
            })
            // All the physical devices that pass the filters above are suitable for the
            // application. However, not every device is equal, some are preferred over others.
            // Now, we assign each physical device a score, and pick the device with the lowest
            // ("best") score.
            //
            // In this example, we simply select the best-scoring device to use in the application.
            // In a real-world setting, you may want to use the best-scoring device only as a
            // "default" or "recommended" device, and let the user choose the device themself.
            .min_by_key(|(p, _)| {
                // We assign a lower score to device types that are likely to be faster/better.
                match p.properties().device_type {
                    PhysicalDeviceType::DiscreteGpu => 0,
                    PhysicalDeviceType::IntegratedGpu => 1,
                    PhysicalDeviceType::VirtualGpu => 2,
                    PhysicalDeviceType::Cpu => 3,
                    PhysicalDeviceType::Other => 4,
                    _ => 5,
                }
            })
            .expect("No suitable physical device found");

        // Print out the device we selected
        println!(
            "Using device: {} (type: {:?})",
            physical_device.properties().device_name,
            physical_device.properties().device_type,
        );

        // Now initializing the device. This is probably the most important object of Vulkan.
        //
        // An iterator of created queues is returned by the function alongside the device. Each
        // queue has a reference to its instance so we don't need to store that directly.
        let (_, mut queues) = Device::new(
            // Which physical device to connect to.
            physical_device,
            DeviceCreateInfo {
                // A list of optional features and extensions that our program needs to work
                // correctly. Some parts of the Vulkan specs are optional and must be enabled
                // manually at device creation. In this example the only thing we are going to need
                // is the `khr_swapchain` extension that allows us to draw to a window.
                enabled_extensions: device_extensions,

                // The list of queues that we are going to use. Here we only use one queue, from
                // the previously chosen queue family.
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],

                ..Default::default()
            },
        )
        .expect("Device initialization failed");

        // Since we can request multiple queues, the `queues` variable is in fact an iterator. We
        // only use one queue in this example, so we just retrieve the first and only element of
        // the iterator.
        queues.next().unwrap()
    }

    fn validation_instance_config(
        library: &Arc<VulkanLibrary>,
        required_extensions: InstanceExtensions,
        validate: bool,
    ) -> (
        InstanceExtensions,
        Vec<String>,
        Vec<DebugUtilsMessengerCreateInfo>,
    ) {
        if !validate {
            return (required_extensions, Vec::new(), Vec::new());
        }

        const VALIDATION_LAYER: &str = "VK_LAYER_KHRONOS_validation";

        let mut enabled_extensions = required_extensions;
        let mut enabled_layers = Vec::new();
        let mut debug_utils_messengers = Vec::new();

        let has_validation_layer = library
            .layer_properties()
            .map(|layers| {
                layers
                    .into_iter()
                    .any(|layer| layer.name() == VALIDATION_LAYER)
            })
            .unwrap_or(false);

        if has_validation_layer {
            enabled_layers.push(VALIDATION_LAYER.to_owned());

            if library.supported_extensions().ext_debug_utils {
                enabled_extensions.ext_debug_utils = true;
                debug_utils_messengers.push(DebugUtilsMessengerCreateInfo {
                    message_severity: DebugUtilsMessageSeverity::ERROR
                        | DebugUtilsMessageSeverity::WARNING
                        | DebugUtilsMessageSeverity::INFO
                        | DebugUtilsMessageSeverity::VERBOSE,
                    message_type: DebugUtilsMessageType::GENERAL
                        | DebugUtilsMessageType::VALIDATION
                        | DebugUtilsMessageType::PERFORMANCE,
                    ..DebugUtilsMessengerCreateInfo::user_callback(unsafe {
                        DebugUtilsMessengerCallback::new(
                            |message_severity, message_type, callback_data| {
                                println!(
                                    "[vulkan {:?} {:?}] {}",
                                    message_severity, message_type, callback_data.message
                                );
                            },
                        )
                    })
                });
                println!("Vulkan validation enabled for vulkan-window example");
            } else {
                eprintln!("Vulkan validation requested, but VK_EXT_debug_utils is not available");
            }
        } else {
            eprintln!(
                "Vulkan validation requested, but '{}' is not available on this system",
                VALIDATION_LAYER,
            );
        }

        (enabled_extensions, enabled_layers, debug_utils_messengers)
    }
}
