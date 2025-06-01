cmake_minimum_required(VERSION 3.31)

project(DirkEngine VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Set build type to Release if not specified (equivalent to -O2)
if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()

find_package(Vulkan QUIET)
find_package(glfw3 REQUIRED)

# Handle Vulkan manually if CMake's FindVulkan fails (common in Nix)
if(NOT Vulkan_FOUND)
    # Try to find Vulkan manually using environment variables
    if(DEFINED ENV{VULKAN_SDK})
        set(Vulkan_INCLUDE_DIRS "$ENV{VULKAN_SDK}/include")
        set(Vulkan_LIBRARIES vulkan)
        set(Vulkan_FOUND TRUE)
        message(STATUS "Found Vulkan manually using VULKAN_SDK: $ENV{VULKAN_SDK}")
    else()
        # Fallback: assume system installation
        find_path(Vulkan_INCLUDE_DIRS vulkan/vulkan.h)
        find_library(Vulkan_LIBRARIES NAMES vulkan vulkan-1)
        
        if(Vulkan_INCLUDE_DIRS AND Vulkan_LIBRARIES)
            set(Vulkan_FOUND TRUE)
            message(STATUS "Found Vulkan manually: ${Vulkan_LIBRARIES}")
        else()
            message(FATAL_ERROR "Could not find Vulkan library")
        endif()
    endif()
endif()

file(GLOB_RECURSE SOURCES "src/*.cpp")

add_executable(${PROJECT_NAME} ${SOURCES})

target_include_directories(${PROJECT_NAME} PRIVATE
    include
    ${Vulkan_INCLUDE_DIRS}
)

target_link_libraries(${PROJECT_NAME} PRIVATE
    ${Vulkan_LIBRARIES}
    glfw
)

set_target_properties(${PROJECT_NAME} PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin"
)

add_custom_target(run
    COMMAND ${PROJECT_NAME}
    DEPENDS ${PROJECT_NAME}
    WORKING_DIRECTORY ${CMAKE_BINARY_DIR}/bin
    COMMENT "Running ${PROJECT_NAME}"
)

set_target_properties(${PROJECT_NAME} PROPERTIES
    CXX_INCLUDE_WHAT_YOU_USE ""
)
