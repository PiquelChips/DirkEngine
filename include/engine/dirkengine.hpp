#include "core.hpp"

#include <cstdint>
#include <string>

class DirkEngine {

public:
    DirkEngine();

private:
    void initWindow();
    void initVulkan();
    void main();
    void tick();
    void deinit();

    const uint32_t WIDTH = 800;
    const uint32_t HEIGHT = 600;

    const std::string NAME = "Dirk Engine";

    GLFWwindow* window = nullptr;
};
