# Platform check
if(APPLE)
    if(NOT CMAKE_GENERATOR STREQUAL "Xcode")
        message(FATAL_ERROR "Building for Apple platforms requires using Xcode generator.")
    endif()
    if(DEFINED PLATFORM AND NOT PLATFORM MATCHES "^(MAC|MAC_ARM64|MAC_CATALYST|MAC_CATALYST_ARM64|OS64|SIMULATOR64|SIMULATORARM64)$")
        message(FATAL_ERROR "CMake iOS toolchain platform '${PLATFORM}' is not supported.")
    endif()
else()
    message(FATAL_ERROR "The current platform is not supported.")
endif()

# Platform languages
if(APPLE)
    set(PLATFORM_LANGUAGES C CXX OBJC OBJCXX Swift)
else()
    set(PLATFORM_LANGUAGES C CXX)
endif()

# Platform functions
include(AddPlatformLibrary)
include(AddPlatformSources)
