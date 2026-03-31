use crate::models::manifest::Component;

#[allow(dead_code)]
pub struct CameraService {
    snapshot_url: String,
    client: reqwest::Client,
}

#[allow(dead_code)]
impl CameraService {
    /// Create a CameraService from a camera component's manifest entry.
    /// Returns None if the component is not a WiFi camera.
    pub fn from_component(component: &Component) -> Option<Self> {
        if component.component_type != "camera" {
            return None;
        }

        // Only WiFi cameras are supported (e.g., ESP32-CAM)
        if component.connection.as_deref() != Some("wifi") {
            return None;
        }

        // The subtype tells us the endpoint format.
        // ESP32-CAM default: http://<ip>/capture
        // The IP must be configured as a pin field or subtype detail.
        // For now, look for a "url" or "ip" field in the pins map.
        // Convention: pin name "stream_ip" holds the IP address as a u8 placeholder,
        // or more practically, the connection field is "wifi" and we look for subtype "esp32-cam".
        // The actual URL will need to be provided by the user in a future settings field.
        // For now, use a placeholder that can be overridden.

        let ip = "192.168.1.100"; // Default ESP32-CAM IP — user should configure this
        let snapshot_url = format!("http://{}:80/capture", ip);

        Some(Self {
            snapshot_url,
            client: reqwest::Client::new(),
        })
    }

    /// Capture a single JPEG frame from the camera.
    pub async fn capture_frame(&self) -> Result<Vec<u8>, String> {
        let response = self
            .client
            .get(&self.snapshot_url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Camera capture failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Camera returned status {}", response.status()));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read camera frame: {}", e))?;

        Ok(bytes.to_vec())
    }

    pub fn snapshot_url(&self) -> &str {
        &self.snapshot_url
    }
}
