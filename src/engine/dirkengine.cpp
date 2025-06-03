#include "engine/dirkengine.hpp"
#include <iostream>

DirkEngine::DirkEngine() {
    initWindow();
    initVulkan();
    main();
    deinit();
}

void DirkEngine::initWindow() {
    glfwInit();

    glfwWindowHint(GLFW_CLIENT_API, GLFW_NO_API);
    glfwWindowHint(GLFW_RESIZABLE, GLFW_FALSE);

    window = glfwCreateWindow(WIDTH, HEIGHT, NAME.c_str(), nullptr, nullptr);
}

void DirkEngine::initVulkan() { std::cout << "Initlializing Vulkan...\n"; }

void DirkEngine::main() {
    while (!glfwWindowShouldClose(window)) {
        glfwPollEvents();

        tick();
    }
}

void DirkEngine::tick() { std::cout << "Tick\n"; }

void DirkEngine::deinit() {
    glfwDestroyWindow(window);

    glfwTerminate();
}
