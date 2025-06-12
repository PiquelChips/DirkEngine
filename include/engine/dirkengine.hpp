#pragma once

#include <GLFW/glfw3.h>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

#include "logger.hpp"
#include "vulkan/vk_platform.h"
#include "vulkan/vulkan_core.h"

#ifdef DEBUG_BUILD
#define ENABLE_VALIDATION_LAYERS
#endif

struct QueueFamilyIndices {
    std::optional<uint32_t> graphicsFamily;

    bool isComplete() {
        return graphicsFamily.has_value();
    }
};

class DirkEngine {

public:
    DirkEngine(Logger* logger);

    bool init();
    void start();
    void exit();

    bool isRequestingExit() const noexcept;

    Logger* getLogger() { return logger; }

private:
    void initWindow();
    void initVulkan();

    std::vector<const char*> getRequiredInstanceExtensions();

    void getPhysicalDevice();
    int getDeviceSuitability(VkPhysicalDevice device);

    QueueFamilyIndices findQueueFamilies(VkPhysicalDevice device);

    void tick();

    void cleanup();

public:
    const uint32_t WIDTH = 800;
    const uint32_t HEIGHT = 600;
    const std::string NAME = "Dirk Engine";

#ifdef ENABLE_VALIDATION_LAYERS
public:
    static VKAPI_ATTR VkBool32 VKAPI_CALL debugCallback(
        VkDebugUtilsMessageSeverityFlagBitsEXT messageSeverity,
        VkDebugUtilsMessageTypeFlagsEXT messageType,
        const VkDebugUtilsMessengerCallbackDataEXT* pCallbackData,
        void* pUserData);

private:
    bool checkValidationLayerSupport();
    void setupDebugMessenger();

    std::vector<const char*> validationLayers = {"VK_LAYER_KHRONOS_validation"};
    VkDebugUtilsMessengerEXT debugMessenger;
#endif

private:
    GLFWwindow* window = nullptr;
    VkInstance instance = nullptr;
    VkPhysicalDevice physicalDevice = VK_NULL_HANDLE;

    Logger* logger = nullptr;

    bool initSuccessful = false;
    bool requestingExit = false;
};
