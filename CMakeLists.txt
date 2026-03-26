# Filename: CMakeLists.txt
# Destination: ./CMakeLists.txt

cmake_minimum_required(VERSION 3.20)
project(ArtemisKernels CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

option(ARTEMIS_BUILD_TESTS "Build unit tests" ON)

add_library(artemis_core
    src/core/metrics.cpp
    src/core/corridors.cpp
    src/core/lyapunov.cpp
)

target_include_directories(artemis_core PUBLIC include)

add_library(hydraulic_kernel
    src/hydraulic/sewer_kernel.cpp
)
target_link_libraries(hydraulic_kernel PUBLIC artemis_core)

add_library(energy_kernel
    src/energy/turbine_kernel.cpp
)
target_link_libraries(energy_kernel PUBLIC artemis_core)

add_library(neuromorphic_kernel
    src/neuromorphic/snn_quality_kernel.cpp
)
target_link_libraries(neuromorphic_kernel PUBLIC artemis_core)

add_library(carbon_health_kernel
    src/carbon_health/furnace_kernel.cpp
)
target_link_libraries(carbon_health_kernel PUBLIC artemis_core)

if(ARTEMIS_BUILD_TESTS)
  enable_testing()
  add_executable(artemis_tests
      tests/test_kernels.cpp
  )
  target_link_libraries(artemis_tests
      PRIVATE artemis_core hydraulic_kernel
              energy_kernel neuromorphic_kernel
              carbon_health_kernel
  )
  add_test(NAME ArtemisKernels COMMAND artemis_tests)
endif()
