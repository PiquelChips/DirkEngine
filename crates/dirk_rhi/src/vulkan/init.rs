use std::{collections::HashSet, ffi::CStr};

use ash::{Entry, vk};
use dirk_utils::Version;
use gpu_allocator::{
    AllocationSizes, AllocatorDebugSettings,
    vulkan::{Allocator, AllocatorCreateDesc},
};
use tracing::info;

use super::{Inner, Queue, Queues, VulkanBackend, VulkanDevice, map_error, unsupported};
use crate::{Device, Result};

/// Application metadata used to initialize Vulkan.
pub struct VulkanCreateInfo {
    /// Engine name reported to Vulkan tooling.
    pub engine_name: std::ffi::CString,
    /// Engine version reported to Vulkan tooling.
    pub engine_version: Version,
    /// Application name reported to Vulkan tooling.
    pub application_name: std::ffi::CString,
    /// Application version reported to Vulkan tooling.
    pub application_version: Version,
}

impl VulkanBackend {
    /// Creates a headless Vulkan device.
    pub fn create(info: &VulkanCreateInfo) -> Result<VulkanDevice> {
        let entry = unsafe { Entry::load() }.map_err(|error| map_error("load Vulkan", error))?;
        let instance = create_instance(&entry, info, required_headless_extensions())?;
        let selected = select_physical_device(instance.instance())?;
        create_backend(entry, instance, selected).map(Device::new)
    }

    /// Creates a Vulkan device selected for presentation to `window`.
    #[cfg(feature = "presentation")]
    pub fn create_for_window(
        info: &VulkanCreateInfo,
        window: &(impl super::VulkanSurfaceTarget + ?Sized),
    ) -> Result<VulkanDevice> {
        let entry = unsafe { Entry::load() }.map_err(|error| map_error("load Vulkan", error))?;
        let display = window
            .display_handle()
            .map_err(|error| map_error("get display handle", error))?;
        let mut extensions = ash_window::enumerate_required_extensions(display.as_raw())
            .map_err(|error| map_error("enumerate window extensions", error))?
            .to_vec();
        extensions.extend(required_headless_extensions());
        deduplicate_extensions(&mut extensions);
        let instance = create_instance(&entry, info, extensions)?;
        let surface_loader = ash::khr::surface::Instance::new(&entry, instance.instance());
        let selected = {
            let surface = TemporarySurface::new(
                &surface_loader,
                super::presentation::create_raw_surface(&entry, instance.instance(), window)?,
            );
            select_physical_device_for_surface(instance.instance(), &surface_loader, surface.raw)?
        };
        create_backend(entry, instance, selected).map(Device::new)
    }
}

impl Device<VulkanBackend> {
    /// Creates a headless Vulkan-backed RHI device.
    pub fn new_vulkan(info: &VulkanCreateInfo) -> Result<Self> {
        VulkanBackend::create(info)
    }

    /// Creates a presentation-ready Vulkan-backed RHI device.
    #[cfg(feature = "presentation")]
    pub fn new_vulkan_for_window(
        info: &VulkanCreateInfo,
        window: &(impl super::VulkanSurfaceTarget + ?Sized),
    ) -> Result<Self> {
        VulkanBackend::create_for_window(info, window)
    }
}

struct InstanceGuard {
    instance: Option<ash::Instance>,
    #[cfg(validation)]
    debug: Option<(ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT)>,
}

impl InstanceGuard {
    fn new(instance: ash::Instance) -> Self {
        Self {
            instance: Some(instance),
            #[cfg(validation)]
            debug: None,
        }
    }

    fn instance(&self) -> &ash::Instance {
        self.instance.as_ref().expect("guard owns Vulkan instance")
    }

    fn into_parts(mut self) -> InstanceParts {
        InstanceParts {
            instance: self.instance.take().expect("guard owns Vulkan instance"),
            #[cfg(validation)]
            debug: self.debug.take().expect("validation messenger was created"),
        }
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        unsafe {
            #[cfg(validation)]
            if let Some((loader, messenger)) = self.debug.take() {
                loader.destroy_debug_utils_messenger(messenger, None);
            }
            if let Some(instance) = self.instance.take() {
                instance.destroy_instance(None);
            }
        }
    }
}

struct InstanceParts {
    instance: ash::Instance,
    #[cfg(validation)]
    debug: (ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT),
}

struct DeviceGuard(Option<ash::Device>);

impl DeviceGuard {
    fn device(&self) -> &ash::Device {
        self.0.as_ref().expect("guard owns Vulkan device")
    }

    fn take(mut self) -> ash::Device {
        self.0.take().expect("guard owns Vulkan device")
    }
}

impl Drop for DeviceGuard {
    fn drop(&mut self) {
        if let Some(device) = self.0.take() {
            unsafe { device.destroy_device(None) };
        }
    }
}

#[cfg(feature = "presentation")]
struct TemporarySurface<'a> {
    loader: &'a ash::khr::surface::Instance,
    raw: vk::SurfaceKHR,
}

#[cfg(feature = "presentation")]
impl<'a> TemporarySurface<'a> {
    fn new(loader: &'a ash::khr::surface::Instance, raw: vk::SurfaceKHR) -> Self {
        Self { loader, raw }
    }
}

#[cfg(feature = "presentation")]
impl Drop for TemporarySurface<'_> {
    fn drop(&mut self) {
        unsafe { self.loader.destroy_surface(self.raw, None) };
    }
}

#[derive(Clone, Copy)]
struct SelectedDevice {
    physical_device: vk::PhysicalDevice,
    queues: QueueFamilies,
}

#[derive(Clone, Copy)]
struct QueueFamilies {
    graphics: u32,
    compute: u32,
    transfer: u32,
    #[cfg(feature = "presentation")]
    present: Option<u32>,
}

fn create_backend(
    entry: Entry,
    instance: InstanceGuard,
    selected: SelectedDevice,
) -> Result<VulkanBackend> {
    let properties = unsafe {
        instance
            .instance()
            .get_physical_device_properties(selected.physical_device)
    };
    let device = DeviceGuard(Some(create_logical_device(instance.instance(), &selected)?));
    let queues = Queues {
        graphics: queue(device.device(), selected.queues.graphics),
        compute: queue(device.device(), selected.queues.compute),
        transfer: queue(device.device(), selected.queues.transfer),
        #[cfg(feature = "presentation")]
        present: selected
            .queues
            .present
            .map(|family| queue(device.device(), family)),
    };
    let allocator = Allocator::new(&AllocatorCreateDesc {
        instance: instance.instance().clone(),
        device: device.device().clone(),
        physical_device: selected.physical_device,
        debug_settings: AllocatorDebugSettings::default(),
        buffer_device_address: false,
        allocation_sizes: AllocationSizes::default(),
    });
    let allocator = allocator.map_err(|error| map_error("create Vulkan allocator", error))?;

    let device = device.take();
    let instance = instance.into_parts();

    #[cfg(feature = "presentation")]
    let surface_loader = ash::khr::surface::Instance::new(&entry, &instance.instance);
    #[cfg(feature = "presentation")]
    let swapchain_loader = ash::khr::swapchain::Device::new(&instance.instance, &device);

    Ok(VulkanBackend {
        inner: std::sync::Arc::new(Inner {
            entry,
            instance: instance.instance,
            device,
            physical_device: selected.physical_device,
            queues,
            queue_lock: std::sync::Mutex::new(()),
            max_sampler_anisotropy: properties.limits.max_sampler_anisotropy,
            allocator: std::sync::Mutex::new(Some(allocator)),
            deletion_queue: std::sync::Mutex::new(Vec::new()),
            #[cfg(validation)]
            debug_loader: instance.debug.0,
            #[cfg(validation)]
            debug_messenger: instance.debug.1,
            #[cfg(feature = "presentation")]
            surface_loader,
            #[cfg(feature = "presentation")]
            swapchain_loader,
        }),
    })
}

fn create_instance(
    entry: &Entry,
    info: &VulkanCreateInfo,
    mut extensions: Vec<*const i8>,
) -> Result<InstanceGuard> {
    #[cfg(validation)]
    extensions.push(ash::ext::debug_utils::NAME.as_ptr());
    deduplicate_extensions(&mut extensions);
    validate_instance_extensions(entry, &extensions)?;
    #[cfg(validation)]
    super::debug::validate_layers(entry)?;

    let application_info = vk::ApplicationInfo::default()
        .application_name(info.application_name.as_c_str())
        .application_version(version(info.application_version))
        .engine_name(info.engine_name.as_c_str())
        .engine_version(version(info.engine_version))
        .api_version(vk::API_VERSION_1_3);
    #[allow(unused_mut)]
    let mut create_info = vk::InstanceCreateInfo::default()
        .application_info(&application_info)
        .enabled_extension_names(&extensions);
    #[cfg(validation)]
    let mut debug_info = super::debug::create_info();
    #[cfg(validation)]
    {
        info!(target: "vulkan::validation", "using validation layers");
        create_info = create_info
            .enabled_layer_names(super::debug::VALIDATION_LAYERS)
            .push_next(&mut debug_info);
    }
    #[cfg(target_os = "macos")]
    {
        create_info = create_info.flags(vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR);
    }

    let raw = unsafe { entry.create_instance(&create_info, None) }
        .map_err(|error| map_error("create Vulkan instance", error))?;
    let mut instance = InstanceGuard::new(raw);
    #[cfg(validation)]
    {
        let loader = ash::ext::debug_utils::Instance::new(entry, instance.instance());
        let messenger = unsafe { loader.create_debug_utils_messenger(&debug_info, None) }
            .map_err(|error| map_error("create Vulkan debug messenger", error))?;
        instance.debug = Some((loader, messenger));
    }
    Ok(instance)
}

fn required_headless_extensions() -> Vec<*const i8> {
    let extensions = Vec::new();
    #[cfg(target_os = "macos")]
    let extensions = {
        let mut extensions = extensions;
        extensions.push(ash::khr::portability_enumeration::NAME.as_ptr());
        extensions
    };
    extensions
}

fn required_device_extensions(enable_presentation: bool) -> Vec<&'static CStr> {
    let extensions = Vec::new();
    #[cfg(feature = "presentation")]
    let extensions = {
        let mut extensions = extensions;
        if enable_presentation {
            extensions.push(ash::khr::swapchain::NAME);
        }
        extensions
    };
    #[cfg(not(feature = "presentation"))]
    let _ = enable_presentation;
    #[cfg(target_os = "macos")]
    let extensions = {
        let mut extensions = extensions;
        extensions.push(ash::khr::portability_subset::NAME);
        extensions
    };
    extensions
}

fn validate_instance_extensions(entry: &Entry, required: &[*const i8]) -> Result<()> {
    let available = unsafe { entry.enumerate_instance_extension_properties(None) }
        .map_err(|error| map_error("enumerate Vulkan instance extensions", error))?;
    for &name in required {
        let name = unsafe { CStr::from_ptr(name) };
        let found = available.iter().any(|extension| {
            (unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) }) == name
        });
        if !found {
            return Err(unsupported(format!(
                "required Vulkan instance extension {} is unavailable",
                name.to_string_lossy()
            )));
        }
    }
    Ok(())
}

fn select_physical_device(instance: &ash::Instance) -> Result<SelectedDevice> {
    select_physical_device_impl(instance, false, |_, _| Ok(None))
}

#[cfg(feature = "presentation")]
fn select_physical_device_for_surface(
    instance: &ash::Instance,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<SelectedDevice> {
    select_physical_device_impl(instance, true, |physical_device, queue_count| {
        for family in 0..queue_count {
            let supported = unsafe {
                surface_loader.get_physical_device_surface_support(physical_device, family, surface)
            }
            .map_err(|error| map_error("query Vulkan presentation support", error))?;
            if supported {
                return Ok(Some(family));
            }
        }
        Ok(None)
    })
}

fn select_physical_device_impl(
    instance: &ash::Instance,
    require_present: bool,
    present_family: impl Fn(vk::PhysicalDevice, u32) -> Result<Option<u32>>,
) -> Result<SelectedDevice> {
    let devices = unsafe { instance.enumerate_physical_devices() }
        .map_err(|error| map_error("enumerate Vulkan physical devices", error))?;
    let required_extensions = required_device_extensions(require_present);
    let mut candidates = Vec::new();

    for physical_device in devices {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };
        if properties.api_version < vk::API_VERSION_1_3 {
            continue;
        }
        if !supports_extensions(instance, physical_device, &required_extensions)? {
            continue;
        }
        if !supports_required_features(instance, physical_device) {
            continue;
        }
        let queue_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        let Some(universal) = find_universal_queue(&queue_properties) else {
            continue;
        };
        #[cfg(feature = "presentation")]
        let present = present_family(
            physical_device,
            u32::try_from(queue_properties.len())
                .map_err(|_| unsupported("Vulkan exposes more than u32::MAX queue families"))?,
        )?;
        #[cfg(feature = "presentation")]
        if require_present && present.is_none() {
            continue;
        }
        #[cfg(not(feature = "presentation"))]
        let _ = (require_present, &present_family);

        let score = match properties.device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => 4,
            vk::PhysicalDeviceType::INTEGRATED_GPU => 3,
            vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
            vk::PhysicalDeviceType::CPU => 1,
            _ => 0,
        };
        candidates.push((
            score,
            properties,
            SelectedDevice {
                physical_device,
                queues: QueueFamilies {
                    graphics: universal,
                    compute: universal,
                    transfer: universal,
                    #[cfg(feature = "presentation")]
                    present,
                },
            },
        ));
    }

    let (_, properties, selected) = candidates
        .into_iter()
        .max_by_key(|(score, properties, _)| (*score, properties.limits.max_image_dimension2_d))
        .ok_or_else(|| {
            unsupported("no Vulkan 1.3 device supports the required features and extensions")
        })?;
    let name = unsafe { CStr::from_ptr(properties.device_name.as_ptr()) };
    info!(device = %name.to_string_lossy(), "selected Vulkan physical device");
    Ok(selected)
}

fn supports_extensions(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    required: &[&CStr],
) -> Result<bool> {
    let available = unsafe { instance.enumerate_device_extension_properties(physical_device) }
        .map_err(|error| map_error("enumerate Vulkan device extensions", error))?;
    Ok(required.iter().all(|required| {
        available.iter().any(|extension| {
            (unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) }) == *required
        })
    }))
}

fn supports_required_features(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> bool {
    let mut vulkan12 = vk::PhysicalDeviceVulkan12Features::default();
    let mut vulkan13 = vk::PhysicalDeviceVulkan13Features::default();
    let mut features = vk::PhysicalDeviceFeatures2::default()
        .push_next(&mut vulkan12)
        .push_next(&mut vulkan13);
    unsafe {
        instance.get_physical_device_features2(physical_device, &mut features);
    }
    features.features.sampler_anisotropy == vk::TRUE
        && vulkan12.timeline_semaphore == vk::TRUE
        && vulkan13.dynamic_rendering == vk::TRUE
        && vulkan13.synchronization2 == vk::TRUE
}

fn create_logical_device(
    instance: &ash::Instance,
    selected: &SelectedDevice,
) -> Result<ash::Device> {
    #[allow(unused_mut)]
    let mut unique_families = HashSet::from([
        selected.queues.graphics,
        selected.queues.compute,
        selected.queues.transfer,
    ]);
    #[cfg(feature = "presentation")]
    if let Some(present) = selected.queues.present {
        unique_families.insert(present);
    }
    let priorities = [1.0_f32];
    let queue_infos = unique_families
        .into_iter()
        .map(|family| {
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(family)
                .queue_priorities(&priorities)
        })
        .collect::<Vec<_>>();
    #[cfg(feature = "presentation")]
    let enable_presentation = selected.queues.present.is_some();
    #[cfg(not(feature = "presentation"))]
    let enable_presentation = false;
    let extension_names = required_device_extensions(enable_presentation)
        .into_iter()
        .map(CStr::as_ptr)
        .collect::<Vec<_>>();
    let mut vulkan12 = vk::PhysicalDeviceVulkan12Features::default().timeline_semaphore(true);
    let mut vulkan13 = vk::PhysicalDeviceVulkan13Features::default()
        .dynamic_rendering(true)
        .synchronization2(true);
    let features = vk::PhysicalDeviceFeatures::default().sampler_anisotropy(true);
    let create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(&extension_names)
        .enabled_features(&features)
        .push_next(&mut vulkan12)
        .push_next(&mut vulkan13);
    unsafe { instance.create_device(selected.physical_device, &create_info, None) }
        .map_err(|error| map_error("create Vulkan logical device", error))
}

fn find_queue(
    properties: &[vk::QueueFamilyProperties],
    required: vk::QueueFlags,
    excluded: vk::QueueFlags,
) -> Option<u32> {
    properties
        .iter()
        .enumerate()
        .find_map(|(index, properties)| {
            (properties.queue_count > 0
                && properties.queue_flags.contains(required)
                && !properties.queue_flags.intersects(excluded))
            .then(|| u32::try_from(index).expect("Vulkan queue-family count fits in u32"))
        })
}

fn find_universal_queue(properties: &[vk::QueueFamilyProperties]) -> Option<u32> {
    find_queue(
        properties,
        vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER,
        vk::QueueFlags::empty(),
    )
}

fn queue(device: &ash::Device, family_index: u32) -> Queue {
    Queue {
        raw: unsafe { device.get_device_queue(family_index, 0) },
        family_index,
    }
}

fn deduplicate_extensions(extensions: &mut Vec<*const i8>) {
    extensions.sort_unstable_by(|left, right| unsafe {
        CStr::from_ptr(*left)
            .to_bytes()
            .cmp(CStr::from_ptr(*right).to_bytes())
    });
    extensions.dedup_by(|left, right| unsafe { CStr::from_ptr(*left) == CStr::from_ptr(*right) });
}

fn version(version: Version) -> u32 {
    vk::make_api_version(0, version.major(), version.minor(), version.patch())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue_family(flags: vk::QueueFlags) -> vk::QueueFamilyProperties {
        vk::QueueFamilyProperties {
            queue_flags: flags,
            queue_count: 1,
            ..Default::default()
        }
    }

    #[test]
    fn universal_queue_requires_compute_support() {
        let properties = [queue_family(
            vk::QueueFlags::GRAPHICS | vk::QueueFlags::TRANSFER,
        )];
        assert_eq!(find_universal_queue(&properties), None);
    }

    #[test]
    fn universal_queue_supports_every_rhi_queue_type() {
        let properties = [
            queue_family(vk::QueueFlags::TRANSFER),
            queue_family(
                vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER,
            ),
        ];
        assert_eq!(find_universal_queue(&properties), Some(1));
    }
}
