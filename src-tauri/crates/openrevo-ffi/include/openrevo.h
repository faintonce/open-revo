#ifndef OPENREVO_H
#define OPENREVO_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include <stddef.h>

/**
 * OpenRevo Core Engine - Native C-ABI Header
 * Version: 0.9.6
 */

/**
 * Hardware asynchronous event callback function signature.
 * @param event_type C-string (e.g. "physical-mode-switched", "fan-boost-changed", "osd-notify")
 * @param json_payload C-string containing JSON-encoded event arguments
 * @param user_data User context pointer
 */
typedef void (*OpenRevoEventCallback)(const char* event_type, const char* json_payload, void* user_data);

/**
 * Initialize the OpenRevo Core Engine.
 * @return 0 on success, -1 on failure.
 */
int32_t openrevo_init(void);

/**
 * Get engine version string. Returned pointer is static and MUST NOT be freed.
 */
const char* openrevo_version(void);

/**
 * Check if the engine runtime is active.
 * @return 1 if active, 0 otherwise.
 */
int32_t openrevo_is_active(void);

/**
 * Gracefully shutdown the engine, signaling all background monitors to exit.
 */
void openrevo_shutdown(void);

/**
 * Register an asynchronous event callback for hardware and telemetry events.
 * @return 0 on success, -1 on error.
 */
int32_t openrevo_register_event_callback(OpenRevoEventCallback callback, void* user_data);

/**
 * Query full hardware snapshot as a JSON C-string.
 * Caller MUST free the returned string with openrevo_free_string.
 */
char* openrevo_get_hardware_snapshot_json(void);

/**
 * Query factory power presets as a JSON C-string.
 * Caller MUST free the returned string with openrevo_free_string.
 */
char* openrevo_get_power_presets_json(void);

/**
 * Set physical performance mode (0 = Office, 1 = Game/Balanced, 2 = Beast/Turbo).
 * @return 0 on success, -1 on failure.
 */
int32_t openrevo_set_power_mode(uint8_t mode);

/**
 * Set telemetry polling heartbeat state.
 * @param active 1 to enable, 0 to pause.
 */
void openrevo_set_telemetry_active(int32_t active);

/**
 * Check if telemetry polling heartbeat is active.
 * @return 1 if active, 0 otherwise.
 */
int32_t openrevo_is_telemetry_active(void);

/**
 * Free a dynamically allocated C-string returned by OpenRevo Core API.
 */
void openrevo_free_string(char* ptr);

#ifdef __cplusplus
}
#endif

#endif // OPENREVO_H

