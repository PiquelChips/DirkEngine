#pragma once

#include <GLFW/glfw3.h>
#include <cstdint>
#include <string>

#include "logger.hpp"

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

    void tick();

    void cleanup();

public:
    const uint32_t WIDTH = 800;
    const uint32_t HEIGHT = 600;
    const std::string NAME = "Dirk Engine";

private:
    GLFWwindow* window = nullptr;

    Logger* logger = nullptr;

    bool initSuccessful = false;
    bool requestingExit = false;
};
