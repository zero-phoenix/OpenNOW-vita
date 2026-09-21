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

set(VITA_ADBD_PAIRING_KEY_HEX "" CACHE STRING "64 hexadecimal characters shared with the PC")
string(LENGTH "${VITA_ADBD_PAIRING_KEY_HEX}" VITA_ADBD_PAIRING_KEY_LENGTH)
if(NOT VITA_ADBD_PAIRING_KEY_LENGTH EQUAL 64 OR NOT VITA_ADBD_PAIRING_KEY_HEX MATCHES "^[0-9a-fA-F]+$")
  message(FATAL_ERROR "Set VITA_ADBD_PAIRING_KEY_HEX to exactly 64 hexadecimal characters")
endif()

# Kernel PRX: no libc, no dynamic allocator, and no exported arbitrary command surface.
set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Wl,-q -Wall -Wextra -Werror -O2 -nostdlib")
add_executable(vita_adb_kernel src/main.c src/sha256.c)
target_compile_definitions(vita_adb_kernel PRIVATE
  VITA_ADBD_PAIRING_KEY_HEX="${VITA_ADBD_PAIRING_KEY_HEX}"
)
target_link_libraries(vita_adb_kernel
  SceUsbSerialForDriver_stub
  SceThreadmgrForDriver_stub
  SceSysclibForDriver_stub
  SceNetPsForDriver_stub
)

vita_create_self(vita-adb.skprx vita_adb_kernel CONFIG exports.yml UNSAFE)
