#include "engine/dirkengine.hpp"

#include <cstdlib>
#include <iostream>
#include <memory>

int main() {
    auto logger = std::make_unique<Logger>();
    auto engine = std::make_unique<DirkEngine>(logger.get());

    try {
        engine->init();

        engine->start();
    } catch (const std::exception& e) {
        std::cerr << e.what() << std::endl;
        return EXIT_FAILURE;
    }

    return EXIT_SUCCESS;
}
