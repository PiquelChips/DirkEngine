#include <GLFW/glfw3.h>
#include <cstdint>
#include <string>

class DirkEngine {

public:
    bool init();
    void start();
    void exit();

    bool isRequestingExit() const noexcept;

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

    bool initSuccessful = false;
    bool requestingExit = false;
};
