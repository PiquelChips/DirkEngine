#include "engine/dirkengine.hpp"
#include "logger.hpp"
#include "vulkan/vulkan_core.h"

#include <GLFW/glfw3.h>
#include <cassert>
#include <cstdint>
#include <cstring>
#include <vector>

DirkEngine::DirkEngine(Logger* logger) : logger(logger) {}

bool DirkEngine::init() {
    initWindow();
    initVulkan();

    // if init successful
    // log something
    initSuccessful = true;
    return initSuccessful;
}

void DirkEngine::initWindow() {
    assert(glfwInit());

    glfwWindowHint(GLFW_CLIENT_API, GLFW_NO_API);
    glfwWindowHint(GLFW_RESIZABLE, GLFW_FALSE);

    window = glfwCreateWindow(WIDTH, HEIGHT, NAME.c_str(), nullptr, nullptr);
    assert(window);
}

void DirkEngine::initVulkan() {
    logger->Get(INFO) << "Initlializing Vulkan...";

    VkApplicationInfo appInfo{};
    appInfo.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
    appInfo.pApplicationName = "DirkEngine";
    appInfo.applicationVersion = VK_MAKE_VERSION(1, 0, 0);
    appInfo.pEngineName = "DirkEngine";
    appInfo.engineVersion = VK_MAKE_VERSION(1, 0, 0);
    appInfo.apiVersion = VK_API_VERSION_1_4;

    VkInstanceCreateInfo createInfo{};
    createInfo.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    createInfo.pApplicationInfo = &appInfo;

    auto instanceExtensions = getRequiredInstanceExtensions();
    createInfo.enabledExtensionCount = instanceExtensions.size();
    createInfo.ppEnabledExtensionNames = instanceExtensions.data();

#ifdef ENABLE_VALIDATION_LAYERS
    assert(checkValidationLayerSupport());
    logger->Get(INFO) << "Using validation layers";

    createInfo.enabledLayerCount = validationLayers.size();
    createInfo.ppEnabledLayerNames = validationLayers.data();
#else
    createInfo.enabledLayerCount = 0;
#endif

    assert(vkCreateInstance(&createInfo, nullptr, &instance) == VK_SUCCESS);

#ifdef ENABLE_VALIDATION_LAYERS
    setupDebugMessenger();
#endif
}

void DirkEngine::start() {
    assert(initSuccessful);

    while (true) {
        if (glfwWindowShouldClose(window))
            return;

        if (isRequestingExit())
            return;

        glfwPollEvents();

        tick();
    }

    cleanup();
}

void DirkEngine::tick() {
    // logger->Get(DEBUG) << "Tick";
}

void DirkEngine::exit() { requestingExit = true; }

void DirkEngine::cleanup() {
#ifdef ENABLE_VALIDATION_LAYERS
    auto func = (PFN_vkDestroyDebugUtilsMessengerEXT)vkGetInstanceProcAddr(instance, "vkDestroyDebugUtilsMessengerEXT");
    assert(func);
    func(instance, debugMessenger, nullptr);
#endif

    vkDestroyInstance(instance, nullptr);

    glfwDestroyWindow(window);
    glfwTerminate();
}

bool DirkEngine::isRequestingExit() const noexcept { return requestingExit; }

std::vector<const char*> DirkEngine::getRequiredInstanceExtensions() {
    uint32_t glfwExtensionCount = 0;
    const char** glfwExtensions;
    glfwExtensions = glfwGetRequiredInstanceExtensions(&glfwExtensionCount);

    std::vector<const char*> extensions(glfwExtensions, glfwExtensions + glfwExtensionCount);

#ifdef ENABLE_VALIDATION_LAYERS
    extensions.push_back(VK_EXT_DEBUG_UTILS_EXTENSION_NAME);
#endif

    return extensions;
}

#ifdef ENABLE_VALIDATION_LAYERS
bool DirkEngine::checkValidationLayerSupport() {
    uint32_t layerCount = 0;
    vkEnumerateInstanceLayerProperties(&layerCount, nullptr);

    std::vector<VkLayerProperties> availableLayers(layerCount);
    vkEnumerateInstanceLayerProperties(&layerCount, availableLayers.data());

    for (const char* layerName : validationLayers) {
        bool layerFound = false;

        for (const auto& layerProperties : availableLayers)
            if (strcmp(layerName, layerProperties.layerName) == 0)
                layerFound = true;

        if (!layerFound) {
            logger->Get(ERROR) << "Validation layer \"" << layerName << "\" not found";
            return false;
        }
    }

    return true;
}

void DirkEngine::setupDebugMessenger() {
    VkDebugUtilsMessengerCreateInfoEXT createInfo{};
    createInfo.sType = VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT;
    createInfo.messageSeverity = VK_DEBUG_UTILS_MESSAGE_SEVERITY_VERBOSE_BIT_EXT | VK_DEBUG_UTILS_MESSAGE_SEVERITY_WARNING_BIT_EXT | VK_DEBUG_UTILS_MESSAGE_SEVERITY_ERROR_BIT_EXT;
    createInfo.messageType = VK_DEBUG_UTILS_MESSAGE_TYPE_GENERAL_BIT_EXT | VK_DEBUG_UTILS_MESSAGE_TYPE_VALIDATION_BIT_EXT | VK_DEBUG_UTILS_MESSAGE_TYPE_PERFORMANCE_BIT_EXT;
    createInfo.pfnUserCallback = debugCallback;
    createInfo.pUserData = this;

    auto func = (PFN_vkCreateDebugUtilsMessengerEXT)vkGetInstanceProcAddr(instance, "vkCreateDebugUtilsMessengerEXT");
    assert(func);
    assert(func(instance, &createInfo, nullptr, &debugMessenger) == VK_SUCCESS);
}
#endif

VkBool32 DirkEngine::debugCallback(
    VkDebugUtilsMessageSeverityFlagBitsEXT messageSeverity,
    VkDebugUtilsMessageTypeFlagsEXT,
    const VkDebugUtilsMessengerCallbackDataEXT* pCallbackData,
    void* pUserData) {

    DirkEngine* engine = static_cast<DirkEngine*>(pUserData);
    assert(engine);
    engine->logger->Get(ERROR) << pCallbackData->pMessage;

    return VK_FALSE;
}
