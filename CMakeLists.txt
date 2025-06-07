cmake_minimum_required(VERSION 3.31)

project(DirkEngine VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(GLFW_BUILD_DOCS OFF CACHE BOOL "" FORCE)
set(GLFW_INSTALL OFF CACHE BOOL "" FORCE)
set(GLFW_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(GLFW_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
set(BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)

add_subdirectory(thirdparty/glfw-3.4)
add_subdirectory(thirdparty/glm)
add_subdirectory(thirdparty/vulkan)

file(GLOB_RECURSE SOURCES CONFIGURE_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/src/*.cpp")

add_executable(${PROJECT_NAME} ${SOURCES})

# Preprocessor macros
target_compile_definitions("${CMAKE_PROJECT_NAME}" PUBLIC
    GLFW_INCLUDE_NONE=1
    GLFW_INCLUDE_VULKAN=1

    RESSOURCE_PATH=$<IF:$<CONFIG:Release>,"./ressources/","${CMAKE_CURRENT_SOURCE_DIR}/resources">

    DEBUG_BUILD=$<CONFIG:Debug>
    RELEASE_BUILD=$<CONFIG:Release>
)

target_include_directories(${PROJECT_NAME} PRIVATE
    include
)

target_link_libraries(${PROJECT_NAME} PRIVATE
    glfw
    glm
    Vulkan::Loader
    Vulkan::Headers
)

add_custom_target(run
    COMMAND ${PROJECT_NAME}
    DEPENDS ${PROJECT_NAME}
    COMMENT "Running ${PROJECT_NAME}"
)
