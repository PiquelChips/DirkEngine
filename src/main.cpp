#include <vulkan/vulkan.h>

#include <cstdlib>
#include <iostream>

#include "engine/dirkengine.hpp"

int main() {
    DirkEngine app;

    std::cout << "Hello World!" << std::endl;

    try {
        app.run();
    } catch (const std::exception& e) {
        std::cerr << e.what() << std::endl;
        return EXIT_FAILURE;
    }

    return EXIT_SUCCESS;
}
