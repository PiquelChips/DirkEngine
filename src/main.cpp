#include <cstdlib>
#include <iostream>

#include "engine/dirkengine.hpp"

int main() {
    DirkEngine* engine = new DirkEngine();

    std::cout << RESSOURCE_PATH << DEBUG_BUILD << RELEASE_BUILD;

    try {
        engine->init();

        engine->start();
    } catch (const std::exception& e) {
        std::cerr << e.what() << std::endl;
        return EXIT_FAILURE;
    }

    std::cout << std::endl;

    return EXIT_SUCCESS;
}
