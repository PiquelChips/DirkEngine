#include <cstdlib>
#include <iostream>

#include "engine/dirkengine.hpp"

int main() {
    Logger* logger = new Logger();
    DirkEngine* engine = new DirkEngine(logger);

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
