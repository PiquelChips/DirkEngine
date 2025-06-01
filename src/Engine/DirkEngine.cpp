#include "Engine/DirkEngine.hpp"

void DirkEngine::run()
{
    initVulkan();
    mainLoop();
    cleanup();
}

void DirkEngine::initVulkan() { }
void DirkEngine::mainLoop() { }
void DirkEngine::cleanup() { }
