cmake_minimum_required(VERSION 3.16)

if(NOT DEFINED CMAKE_TOOLCHAIN_FILE)
  if(DEFINED ENV{VITASDK})
    set(CMAKE_TOOLCHAIN_FILE "$ENV{VITASDK}/share/vita.toolchain.cmake" CACHE PATH "")
  else()
    message(FATAL_ERROR "VITASDK is required")
  endif()
endif()

project(vita_adb_kernel C)
include("${VITASDK}/share/vita.cmake" REQUIRED)

# Kernel PRX: no libc, no dynamic allocator, and no exported arbitrary command surface.
set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Wl,-q -Wall -Wextra -Werror -O2 -nostdlib")
add_executable(vita_adb_kernel src/main.c)
target_link_libraries(vita_adb_kernel
  SceUsbSerialForDriver_stub
  SceThreadmgrForDriver_stub
  SceSysclibForDriver_stub
)

vita_create_self(vita-adb.skprx vita_adb_kernel CONFIG exports.yml UNSAFE)
