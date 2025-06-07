#include "engine/dirkengine.hpp"

#include <GLFW/glfw3.h>
#include <cassert>

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
    glfwInit();

    glfwWindowHint(GLFW_CLIENT_API, GLFW_NO_API);
    glfwWindowHint(GLFW_RESIZABLE, GLFW_FALSE);

    window = glfwCreateWindow(WIDTH, HEIGHT, NAME.c_str(), nullptr, nullptr);
    assert(window);
}

void DirkEngine::initVulkan() {
    logger->Get(INFO) << "Initlializing Vulkan...";
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
    glfwDestroyWindow(window);

    glfwTerminate();
}

bool DirkEngine::isRequestingExit() const noexcept { return requestingExit; }
