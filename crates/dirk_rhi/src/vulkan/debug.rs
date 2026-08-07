use std::{ffi::CStr, os::raw::c_void};

use ash::{Entry, vk};
use tracing::{debug, error, info, trace, warn};

use super::{map_error, unsupported};
use crate::Result;

pub(super) const VALIDATION_LAYERS: &[*const i8] = &[c"VK_LAYER_KHRONOS_validation".as_ptr()];

unsafe extern "system" fn callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut c_void,
) -> vk::Bool32 {
    let message = unsafe { CStr::from_ptr((*callback_data).p_message) }.to_string_lossy();
    match severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!(target: "vulkan::validation", "{message}");
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!(target: "vulkan::validation", "{message}");
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info!(target: "vulkan::validation", "{message}");
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            debug!(target: "vulkan::validation", "{message}");
        }
        _ => trace!(target: "vulkan::validation", "{message}"),
    }
    vk::FALSE
}

pub(super) fn create_info() -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
    vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        )
        .pfn_user_callback(Some(callback))
}

pub(super) fn validate_layers(entry: &Entry) -> Result<()> {
    let available = unsafe { entry.enumerate_instance_layer_properties() }
        .map_err(|error| map_error("enumerate Vulkan instance layers", error))?;
    for &required in VALIDATION_LAYERS {
        let required = unsafe { CStr::from_ptr(required) };
        if !available
            .iter()
            .any(|layer| (unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) }) == required)
        {
            return Err(unsupported(format!(
                "required Vulkan validation layer {} is unavailable",
                required.to_string_lossy()
            )));
        }
    }
    Ok(())
}
