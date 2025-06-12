#include "engine/dirkengine.hpp"
#include "logger.hpp"
#include "vulkan/vulkan_core.h"

#include <GLFW/glfw3.h>
#include <cassert>
#include <cstdint>
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

    uint32_t glfwExtensionCount = 0;
    const char** glfwExtensions;

    glfwExtensions = glfwGetRequiredInstanceExtensions(&glfwExtensionCount);

    createInfo.enabledExtensionCount = glfwExtensionCount;
    createInfo.ppEnabledExtensionNames = glfwExtensions;

    createInfo.enabledLayerCount = 0;

    assert(vkCreateInstance(&createInfo, nullptr, &instance) == VK_SUCCESS);

    uint32_t extensionCount = 0;
    vkEnumerateInstanceExtensionProperties(nullptr, &extensionCount, nullptr);
    std::vector<VkExtensionProperties> extensions(extensionCount);
    vkEnumerateInstanceExtensionProperties(nullptr, &extensionCount, extensions.data());
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
    vkDestroyInstance(instance, nullptr);

    glfwDestroyWindow(window);
    glfwTerminate();
}

bool DirkEngine::isRequestingExit() const noexcept { return requestingExit; }
