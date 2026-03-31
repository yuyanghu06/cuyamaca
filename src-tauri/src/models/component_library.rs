use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinTemplate {
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentTemplate {
    pub component_type: &'static str,
    pub category: &'static str,
    pub label: &'static str,
    pub pins: Vec<PinTemplate>,
    pub optional_fields: Vec<&'static str>,
    pub serial_output: Option<&'static str>,
}

pub fn get_component_library() -> Vec<ComponentTemplate> {
    vec![
        // ── Actuators ──
        ComponentTemplate {
            component_type: "dc_motor",
            category: "actuator",
            label: "DC Motor",
            pins: vec![
                PinTemplate { name: "pwm", description: "PWM speed control" },
                PinTemplate { name: "dir_a", description: "Direction pin A" },
                PinTemplate { name: "dir_b", description: "Direction pin B" },
            ],
            optional_fields: vec![],
            serial_output: None,
        },
        ComponentTemplate {
            component_type: "servo",
            category: "actuator",
            label: "Servo",
            pins: vec![
                PinTemplate { name: "signal", description: "Signal pin" },
            ],
            optional_fields: vec![],
            serial_output: None,
        },
        ComponentTemplate {
            component_type: "stepper_motor",
            category: "actuator",
            label: "Stepper Motor",
            pins: vec![
                PinTemplate { name: "step", description: "Step pin" },
                PinTemplate { name: "direction", description: "Direction pin" },
            ],
            optional_fields: vec![],
            serial_output: None,
        },
        ComponentTemplate {
            component_type: "relay",
            category: "actuator",
            label: "Relay",
            pins: vec![
                PinTemplate { name: "pin", description: "Control pin" },
            ],
            optional_fields: vec![],
            serial_output: None,
        },
        ComponentTemplate {
            component_type: "led",
            category: "actuator",
            label: "LED",
            pins: vec![
                PinTemplate { name: "pin", description: "Digital or PWM pin" },
            ],
            optional_fields: vec![],
            serial_output: None,
        },
        // ── Distance / Proximity ──
        ComponentTemplate {
            component_type: "ultrasonic",
            category: "distance",
            label: "Ultrasonic (HC-SR04)",
            pins: vec![
                PinTemplate { name: "trig", description: "Trigger pin" },
                PinTemplate { name: "echo", description: "Echo pin" },
            ],
            optional_fields: vec![],
            serial_output: Some("SENSOR_ID:cm"),
        },
        ComponentTemplate {
            component_type: "ir_distance",
            category: "distance",
            label: "IR Distance",
            pins: vec![
                PinTemplate { name: "analog", description: "Analog input pin" },
            ],
            optional_fields: vec![],
            serial_output: Some("SENSOR_ID:0-1023"),
        },
        ComponentTemplate {
            component_type: "lidar_serial",
            category: "distance",
            label: "LiDAR Serial (TF-Mini)",
            pins: vec![
                PinTemplate { name: "rx", description: "RX pin" },
                PinTemplate { name: "tx", description: "TX pin" },
            ],
            optional_fields: vec![],
            serial_output: Some("SENSOR_ID:cm,strength"),
        },
        // ── Motion / Orientation ──
        ComponentTemplate {
            component_type: "imu",
            category: "motion",
            label: "IMU (MPU-6050)",
            pins: vec![
                PinTemplate { name: "sda", description: "I2C data" },
                PinTemplate { name: "scl", description: "I2C clock" },
            ],
            optional_fields: vec![],
            serial_output: Some("ACCEL:x,y,z GYRO:x,y,z"),
        },
        ComponentTemplate {
            component_type: "magnetometer",
            category: "motion",
            label: "Magnetometer (HMC5883L)",
            pins: vec![
                PinTemplate { name: "sda", description: "I2C data" },
                PinTemplate { name: "scl", description: "I2C clock" },
            ],
            optional_fields: vec![],
            serial_output: Some("HEADING:degrees"),
        },
        ComponentTemplate {
            component_type: "encoder",
            category: "motion",
            label: "Rotary Encoder",
            pins: vec![
                PinTemplate { name: "pin_a", description: "Channel A" },
                PinTemplate { name: "pin_b", description: "Channel B" },
            ],
            optional_fields: vec![],
            serial_output: Some("SENSOR_ID:ticks"),
        },
        // ── Touch / Tactile ──
        ComponentTemplate {
            component_type: "bump_switch",
            category: "touch",
            label: "Bump Switch",
            pins: vec![
                PinTemplate { name: "pin", description: "Digital input pin" },
            ],
            optional_fields: vec![],
            serial_output: Some("SENSOR_ID:0 or SENSOR_ID:1"),
        },
        ComponentTemplate {
            component_type: "line_sensor_array",
            category: "touch",
            label: "Line Sensor Array",
            pins: vec![
                PinTemplate { name: "pin_1", description: "Sensor 1" },
                PinTemplate { name: "pin_2", description: "Sensor 2" },
                PinTemplate { name: "pin_3", description: "Sensor 3" },
                PinTemplate { name: "pin_4", description: "Sensor 4" },
                PinTemplate { name: "pin_5", description: "Sensor 5" },
                PinTemplate { name: "pin_6", description: "Sensor 6" },
                PinTemplate { name: "pin_7", description: "Sensor 7" },
                PinTemplate { name: "pin_8", description: "Sensor 8" },
            ],
            optional_fields: vec![],
            serial_output: Some("SENSOR_ID:binary_string"),
        },
        ComponentTemplate {
            component_type: "force_sensor",
            category: "touch",
            label: "Force Sensor",
            pins: vec![
                PinTemplate { name: "analog", description: "Analog input pin" },
            ],
            optional_fields: vec![],
            serial_output: Some("SENSOR_ID:0-1023"),
        },
        // ── Environmental ──
        ComponentTemplate {
            component_type: "temp_humidity",
            category: "environmental",
            label: "Temp/Humidity (DHT22)",
            pins: vec![
                PinTemplate { name: "data", description: "Data pin" },
            ],
            optional_fields: vec![],
            serial_output: Some("TEMP:c HUM:pct"),
        },
        ComponentTemplate {
            component_type: "barometer",
            category: "environmental",
            label: "Barometer (BMP280)",
            pins: vec![
                PinTemplate { name: "sda", description: "I2C data" },
                PinTemplate { name: "scl", description: "I2C clock" },
            ],
            optional_fields: vec![],
            serial_output: Some("PRES:hpa ALT:m"),
        },
        ComponentTemplate {
            component_type: "light",
            category: "environmental",
            label: "Light Sensor (BH1750)",
            pins: vec![
                PinTemplate { name: "sda", description: "I2C data" },
                PinTemplate { name: "scl", description: "I2C clock" },
            ],
            optional_fields: vec![],
            serial_output: Some("LUX:value"),
        },
        ComponentTemplate {
            component_type: "gas",
            category: "environmental",
            label: "Gas Sensor (MQ Series)",
            pins: vec![
                PinTemplate { name: "analog", description: "Analog input pin" },
            ],
            optional_fields: vec![],
            serial_output: Some("SENSOR_ID:ppm"),
        },
        // ── Vision ──
        ComponentTemplate {
            component_type: "camera",
            category: "vision",
            label: "Camera (ESP32-CAM)",
            pins: vec![],
            optional_fields: vec!["subtype", "connection", "resolution", "format"],
            serial_output: None,
        },
    ]
}
