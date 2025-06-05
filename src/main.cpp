#include "engine.hpp"

#include <cstdlib>
#include <iostream>

#include "engine/dirkengine.hpp"

int main() {
    try {
        DirkEngine();
    } catch (const std::exception& e) {
        std::cerr << e.what() << std::endl;
        return EXIT_FAILURE;
    }

    std::cout << std::endl;

    return EXIT_SUCCESS;
}
