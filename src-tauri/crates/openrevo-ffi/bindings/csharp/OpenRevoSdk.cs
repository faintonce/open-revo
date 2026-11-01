using System;
using System.Runtime.InteropServices;
using System.Text;

namespace OpenRevo.Sdk
{
    /// <summary>
    /// Official Native SDK wrapper for OpenRevo Core Engine.
    /// Interoperates directly with `openrevo_core.dll` via C-ABI P/Invoke.
    /// </summary>
    public static class OpenRevoSdk
    {
        private const string DllName = "openrevo_core";

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void NativeEventCallback(IntPtr eventTypePtr, IntPtr jsonPayloadPtr, IntPtr userData);

        #region Native P/Invoke Declarations

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "openrevo_init")]
        private static extern int NativeInit();

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "openrevo_version")]
        private static extern IntPtr NativeVersion();

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "openrevo_is_active")]
        private static extern int NativeIsActive();

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "openrevo_shutdown")]
        private static extern void NativeShutdown();

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "openrevo_register_event_callback")]
        private static extern int NativeRegisterEventCallback(NativeEventCallback callback, IntPtr userData);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "openrevo_get_hardware_snapshot_json")]
        private static extern IntPtr NativeGetHardwareSnapshotJson();

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "openrevo_get_power_presets_json")]
        private static extern IntPtr NativeGetPowerPresetsJson();

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "openrevo_set_power_mode")]
        private static extern int NativeSetPowerMode(byte mode);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "openrevo_set_telemetry_active")]
        private static extern void NativeSetTelemetryActive(int active);

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "openrevo_is_telemetry_active")]
        private static extern int NativeIsTelemetryActive();

        [DllImport(DllName, CallingConvention = CallingConvention.Cdecl, EntryPoint = "openrevo_free_string")]
        private static extern void NativeFreeString(IntPtr ptr);

        #endregion

        // Keep callback reference alive to prevent GC garbage collection
        private static NativeEventCallback _internalCallback;

        /// <summary>
        /// Fires when hardware events occur (e.g. Q-key switch, fan boost switch, telemetry tick).
        /// Arg 1: Event Type (string), Arg 2: JSON Payload (string)
        /// </summary>
        public static event Action<string, string> OnEvent;

        /// <summary>
        /// Initialize the engine and wire up event callbacks.
        /// </summary>
        public static bool Initialize()
        {
            if (NativeInit() != 0) return false;

            _internalCallback = (eventTypePtr, jsonPayloadPtr, _) =>
            {
                string eventType = PtrToStringUtf8(eventTypePtr);
                string jsonPayload = PtrToStringUtf8(jsonPayloadPtr);
                OnEvent?.Invoke(eventType, jsonPayload);
            };

            NativeRegisterEventCallback(_internalCallback, IntPtr.Zero);
            return true;
        }

        public static string Version => PtrToStringUtf8(NativeVersion());

        public static bool IsActive => NativeIsActive() != 0;

        public static void Shutdown() => NativeShutdown();

        public static bool SetPowerMode(byte mode) => NativeSetPowerMode(mode) == 0;

        public static void SetTelemetryActive(bool active) => NativeSetTelemetryActive(active ? 1 : 0);

        public static bool IsTelemetryActive => NativeIsTelemetryActive() != 0;

        /// <summary>
        /// Query full hardware details as a JSON string.
        /// </summary>
        public static string GetHardwareSnapshotJson()
        {
            IntPtr ptr = NativeGetHardwareSnapshotJson();
            return ConsumeAndFreeString(ptr);
        }

        /// <summary>
        /// Query cached factory power presets as a JSON string.
        /// </summary>
        public static string GetPowerPresetsJson()
        {
            IntPtr ptr = NativeGetPowerPresetsJson();
            return ConsumeAndFreeString(ptr);
        }

        #region Helpers

        private static string PtrToStringUtf8(IntPtr ptr)
        {
            if (ptr == IntPtr.Zero) return string.Empty;
#if NETSTANDARD2_1_OR_GREATER || NETCOREAPP1_1_OR_GREATER
            return Marshal.PtrToStringUTF8(ptr) ?? string.Empty;
#else
            int len = 0;
            while (Marshal.ReadByte(ptr, len) != 0) len++;
            byte[] buffer = new byte[len];
            Marshal.Copy(ptr, buffer, 0, len);
            return Encoding.UTF8.GetString(buffer);
#endif
        }

        private static string ConsumeAndFreeString(IntPtr ptr)
        {
            if (ptr == IntPtr.Zero) return string.Empty;
            try
            {
                return PtrToStringUtf8(ptr);
            }
            finally
            {
                NativeFreeString(ptr);
            }
        }

        #endregion
    }
}

