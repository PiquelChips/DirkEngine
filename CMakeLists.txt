cmake_minimum_required(VERSION 3.31)

project(DirkEngine VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

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

file(GLOB_RECURSE SOURCES CONFIGURE_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/src/*.cpp")

add_executable(${PROJECT_NAME} ${SOURCES})

# Preprocessor macros
target_compile_definitions("${CMAKE_PROJECT_NAME}" PUBLIC
    GLFW_INCLUDE_NONE=1
    GLFW_INCLUDE_VULKAN=1

    RESSOURCE_PATH=$<IF:$<CONFIG:Release>,"./ressources","${CMAKE_CURRENT_SOURCE_DIR}/resources">
    LOG_PATH=$<IF:$<CONFIG:Release>,"./logs","${CMAKE_CURRENT_SOURCE_DIR}/logs">

    DEBUG_BUILD=$<CONFIG:Debug>
    RELEASE_BUILD=$<CONFIG:Release>
)

target_include_directories(${PROJECT_NAME} PRIVATE
    include
    ${Vulkan_INCLUDE_DIRS}
)

target_link_libraries(${PROJECT_NAME} PRIVATE
    ${Vulkan_LIBRARIES}
    glfw
)
