cmake_minimum_required(VERSION 3.20)
project(artemis_corridor_kernel VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

set(CMAKE_CXX_FLAGS_RELEASE "-O3 -DNDEBUG -march=native")
set(CMAKE_CXX_FLAGS_DEBUG "-g -O0 -DDEBUG -fsanitize=address,undefined")
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -Wpedantic -Werror")

set(KERNEL_SOURCES
    CorridorSynergyKernel.cpp
)

set(KERNEL_HEADERS
    CorridorSynergyKernel.hpp
)

add_library(artemis_corridor_kernel SHARED ${KERNEL_SOURCES} ${KERNEL_HEADERS})
add_library(artemis::corridor_kernel ALIAS artemis_corridor_kernel)

target_include_directories(artemis_corridor_kernel
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}>
        $<INSTALL_INTERFACE:include>
)

target_compile_features(artemis_corridor_kernel PUBLIC cxx_std_17)

set_target_properties(artemis_corridor_kernel PROPERTIES
    VERSION ${PROJECT_VERSION}
    SOVERSION 1
    OUTPUT_NAME artemis_corridor_kernel
    EXPORT_NAME corridor_kernel
    CXX_VISIBILITY_PRESET hidden
    VISIBILITY_INLINES_HIDDEN ON
)

option(BUILD_TESTS "Build unit tests" OFF)
option(BUILD_BENCHMARKS "Build benchmarks" OFF)
option(ENABLE_SANITIZERS "Enable address and undefined behavior sanitizers" OFF)

if(BUILD_TESTS)
    enable_testing()
    add_executable(kernel_tests tests/kernel_tests.cpp)
    target_link_libraries(kernel_tests PRIVATE artemis::corridor_kernel)
    target_compile_features(kernel_tests PRIVATE cxx_std_17)
    add_test(NAME kernel_tests COMMAND kernel_tests)
endif()

if(BUILD_BENCHMARKS)
    find_package(benchmark REQUIRED)
    add_executable(kernel_benchmarks benchmarks/kernel_benchmarks.cpp)
    target_link_libraries(kernel_benchmarks PRIVATE artemis::corridor_kernel benchmark::benchmark)
    target_compile_features(kernel_benchmarks PRIVATE cxx_std_17)
endif()

if(ENABLE_SANITIZERS)
    target_compile_options(artemis_corridor_kernel PRIVATE -fsanitize=address,undefined)
    target_link_options(artemis_corridor_kernel PRIVATE -fsanitize=address,undefined)
endif()

include(GNUInstallDirs)

install(TARGETS artemis_corridor_kernel
    EXPORT ArtemisKernelTargets
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
)

install(FILES ${KERNEL_HEADERS}
    DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}/artemis/corridor
)

install(EXPORT ArtemisKernelTargets
    FILE ArtemisKernelTargets.cmake
    NAMESPACE artemis::
    DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/artemis_corridor_kernel
)

include(CMakePackageConfigHelpers)

configure_package_config_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/ArtemisKernelConfig.cmake.in
    ${CMAKE_CURRENT_BINARY_DIR}/ArtemisKernelConfig.cmake
    INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/artemis_corridor_kernel
)

write_basic_package_version_file(
    ${CMAKE_CURRENT_BINARY_DIR}/ArtemisKernelConfigVersion.cmake
    VERSION ${PROJECT_VERSION}
    COMPATIBILITY SameMajorVersion
)

install(FILES
    ${CMAKE_CURRENT_BINARY_DIR}/ArtemisKernelConfig.cmake
    ${CMAKE_CURRENT_BINARY_DIR}/ArtemisKernelConfigVersion.cmake
    DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/artemis_corridor_kernel
)

set(CPACK_PACKAGE_NAME "artemis-corridor-kernel")
set(CPACK_PACKAGE_VERSION ${PROJECT_VERSION})
set(CPACK_PACKAGE_VENDOR "Artemis EcoNet Governance")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Artemis Corridor Synergy Kernel - bounded eco-infrastructure optimization")
set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_CURRENT_SOURCE_DIR}/LICENSE")
set(CPACK_RESOURCE_FILE_README "${CMAKE_CURRENT_SOURCE_DIR}/README.md")
set(CPACK_PACKAGE_HOMEPAGE_URL "https://github.com/Doctor0Evil/vsc-artemis-quantum")
set(CPACK_PACKAGE_CHECKSUM SHA256)

include(CPack)

message(STATUS "Artemis Corridor Kernel ${PROJECT_VERSION}")
message(STATUS "C++ Standard: ${CMAKE_CXX_STANDARD}")
message(STATUS "Build Type: ${CMAKE_BUILD_TYPE}")
message(STATUS "Build Tests: ${BUILD_TESTS}")
message(STATUS "Build Benchmarks: ${BUILD_BENCHMARKS}")
message(STATUS "Enable Sanitizers: ${ENABLE_SANITIZERS}")
