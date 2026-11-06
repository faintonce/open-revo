// OpenRevo Hardware Abstraction Layer (HAL) Traits
// Decouples high-level UI & command logic from physical EC RAM / WMI / IOCTL primitives.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwarePowerMode {
    Office = 1,
    Balance = 2,
    Turbo = 3,
    Custom = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatteryHealthMode {
    LongLife = 1, // 100% capacity
    Balance = 2,  // 80% capacity
    Station = 3,  // 60% capacity
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceSwitchType {
    WinKeyLock,
    Touchpad,
    CameraPower,
    FnLock,
    AcPowerRecovery,
    PowerOffUsbCharge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareStatusSnapshot {
    pub power_mode: HardwarePowerMode,
    pub fan_boost: bool,
    pub battery_mode: BatteryHealthMode,
    pub cpu_fan_rpm: u16,
    pub gpu_fan_rpm: u16,
    pub win_key_locked: bool,
    pub usb_charge_active: bool,
    pub ac_recovery_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HalError {
    DriverUnavailable(String),
    RegisterAccessFailed(String),
    UnsupportedModel(String),
    InvalidParameter(String),
    ConflictDetected(String),
}

impl std::fmt::Display for HalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HalError::DriverUnavailable(s) => write!(f, "Driver unavailable: {}", s),
            HalError::RegisterAccessFailed(s) => write!(f, "Register access failed: {}", s),
            HalError::UnsupportedModel(s) => write!(f, "Unsupported model: {}", s),
            HalError::InvalidParameter(s) => write!(f, "Invalid parameter: {}", s),
            HalError::ConflictDetected(s) => write!(f, "Conflict detected: {}", s),
        }
    }
}

impl std::error::Error for HalError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriverKind {
    TongfangAcpi,
    BitlandWmi,
    GenericWmi,
    Mock,
}

pub trait MechrevoHardwareHAL: Send + Sync {
    /// Set machine operating power profile (PL1/PL2, fan profiles, EC trigger)
    fn set_performance_mode(&self, mode: HardwarePowerMode) -> Result<(), HalError>;

    /// Set battery charging health management threshold
    fn set_battery_health(&self, mode: BatteryHealthMode) -> Result<(), HalError>;

    /// Toggle hardware Fan Boost (max cold fan curve override)
    fn set_fan_boost(&self, active: bool) -> Result<(), HalError>;

    /// Toggle peripheral hardware switch
    fn set_device_switch(&self, switch: DeviceSwitchType, enable: bool) -> Result<(), HalError>;

    /// Read dual fan RPM (CPU, GPU)
    fn read_fan_rpm(&self) -> Result<(u16, u16), HalError>;

    /// Set monochrome keyboard backlight level (0..=3 or 0..=4)
    fn set_monochrome_backlight(&self, level: u8) -> Result<(), HalError> {
        let _ = level;
        Err(HalError::UnsupportedModel("Monochrome backlight not supported on this model".into()))
    }

    /// Read monochrome keyboard backlight level if supported by active driver
    fn get_monochrome_backlight(&self) -> Option<u8> {
        None
    }

    /// Query if underlying hardware driver/EC is accessible
    fn is_hardware_supported(&self) -> bool;
}

/// Universal Hardware Driver Provider Trait.
/// Represents a hardware communication backend for a specific laptop platform/ODM (Tongfang, Bitland, etc.).
pub trait HardwareDriver: MechrevoHardwareHAL + Send + Sync {
    fn set_rpm_fan_curves(&self, _curves: crate::hal::fan_policy::RpmFanCurves) -> Result<(), HalError> {
        Err(HalError::UnsupportedModel("RPM fan curves not supported".into()))
    }
    fn set_rpm_fan_smoothing(&self, _smoothing: crate::hal::fan_policy::RpmSmoothing) -> Result<(), HalError> {
        Err(HalError::UnsupportedModel("No software RPM transition control".into()))
    }
    fn set_manual_fan_rpm(&self, _targets: [u16; 2]) -> Result<(), HalError> {
        Err(HalError::UnsupportedModel("Manual fan RPM not supported".into()))
    }
    fn set_software_fan_curve(&self, _curve: Vec<(u8, u8)>) -> Result<(), HalError> {
        Err(HalError::UnsupportedModel("Software fan curve not supported".into()))
    }
    fn restore_fan_auto(&self) -> Result<(), HalError> {
        Err(HalError::UnsupportedModel("Fan auto restore not supported".into()))
    }

    /// Human-readable driver provider name (e.g. "Tongfang-ACPI", "Bitland-WMI")
    fn name(&self) -> &'static str;

    /// Specific driver architecture family
    fn kind(&self) -> DriverKind;

    /// Probe if current machine hardware matches and is supported by this driver
    fn probe(&self) -> bool;

    /// Activate the CPU backend's custom package power profile. Unsupported by default.
    fn apply_cpu_power_profile(&self, _pl1: u16, _pl2: u16) -> Result<(), HalError> {
        Err(HalError::UnsupportedModel("CPU package power backend is not declared".into()))
    }

    /// Edit an existing CPU power session, without changing firmware mode.
    fn update_cpu_power_limits(&self, _pl1: u16, _pl2: u16) -> Result<(), HalError> {
        Err(HalError::UnsupportedModel("CPU package power backend is not declared".into()))
    }

    /// Verify CPU limits, commit settings under the power owner lock, and
    /// roll back hardware if persistence fails. Live edits cannot reactivate it.
    fn commit_cpu_power_profile(&self, _pl1: u16, _pl2: u16, _active_only: bool,
        _persist: &mut dyn FnMut() -> Result<(), String>) -> Result<(), HalError> {
        Err(HalError::UnsupportedModel("Transactional CPU control is not supported".into()))
    }

    /// Read active performance mode
    fn get_performance_mode(&self) -> Result<HardwarePowerMode, HalError>;

    /// Read CPU package temperature in Celsius
    fn read_cpu_temp(&self) -> Result<u8, HalError>;

    /// Read a physical CPU sensor without cached or synthetic fallback values.
    fn read_cpu_temp_sensor(&self) -> Result<u8, HalError> {
        Err(HalError::UnsupportedModel("CPU temperature sensor not implemented on this driver".into()))
    }

    /// Read active GPU MUX switch mode ("hybrid" / "dgpu" / "igpu")
    fn get_gpu_mux(&self) -> Result<String, HalError> {
        Err(HalError::UnsupportedModel("GPU MUX reading not implemented on this driver".into()))
    }

    /// Set discrete GPU MUX switch mode (Hybrid / Discrete-Only)
    fn set_gpu_mux(&self, discrete_only: bool) -> Result<(), HalError> {
        let _ = discrete_only;
        Err(HalError::UnsupportedModel("GPU MUX not implemented on this driver".into()))
    }

    /// Set single/multi-zone keyboard RGB color
    fn set_keyboard_rgb(&self, r: u8, g: u8, b: u8) -> Result<(), HalError> {
        let (_, _, _) = (r, g, b);
        Err(HalError::UnsupportedModel("Keyboard RGB not implemented on this driver".into()))
    }

    /// Set single/multi-zone keyboard effect mode (1=Static/Fixed, 2=Rainbow/Cyclic, etc.)
    fn set_keyboard_effect(&self, effect: u8, r: u8, g: u8, b: u8) -> Result<(), HalError> {
        let _ = effect;
        self.set_keyboard_rgb(r, g, b)
    }
}
