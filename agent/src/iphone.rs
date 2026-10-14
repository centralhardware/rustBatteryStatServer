//! iPhones (and iPads) plugged into this Mac by USB: reads their
//! AppleSmartBattery through usbmuxd → lockdownd → diagnostics_relay, the same
//! path as `idevicediagnostics ioregentry AppleSmartBattery`. The device must
//! have trusted this Mac (the "Trust This Computer?" prompt); usbmuxd hands out
//! the pairing record, so nothing extra is installed. Wi-Fi-synced devices are
//! skipped: only a USB connection is wanted.

use idevice::IdeviceService;
use idevice::services::diagnostics_relay::DiagnosticsRelayClient;
use idevice::usbmuxd::{Connection, UsbmuxdAddr, UsbmuxdConnection};
use serde_json::{Map, Value};
use std::time::Duration;

/// A locked or slow device must not stall the Mac's own sampling.
const TIMEOUT: Duration = Duration::from_secs(10);
const LABEL: &str = "battery-agent";

/// One connected device: its UDID (used as `device_id`) and its battery
/// properties, or why they couldn't be read.
pub struct Reading {
    pub udid: String,
    pub battery: Result<Map<String, Value>, String>,
}

pub struct Reader(tokio::runtime::Runtime);

impl Reader {
    pub fn new() -> Result<Self, String> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map(Reader)
            .map_err(|e| format!("tokio runtime: {e}"))
    }

    /// Every USB-connected device. No usbmuxd (or no devices) is an empty list.
    pub fn read_all(&self) -> Vec<Reading> {
        self.0.block_on(async {
            let Ok(Ok(udids)) = tokio::time::timeout(TIMEOUT, usb_devices()).await else {
                return Vec::new();
            };
            let mut readings = Vec::new();
            for (udid, device) in udids {
                let battery = match tokio::time::timeout(TIMEOUT, battery(device)).await {
                    Ok(r) => r,
                    Err(_) => Err("timed out".into()),
                };
                readings.push(Reading { udid, battery });
            }
            readings
        })
    }
}

async fn usb_devices() -> Result<Vec<(String, idevice::usbmuxd::UsbmuxdDevice)>, String> {
    let mut mux = UsbmuxdConnection::default().await.map_err(|e| e.to_string())?;
    let devices = mux.get_devices().await.map_err(|e| e.to_string())?;
    Ok(devices
        .into_iter()
        .filter(|d| d.connection_type == Connection::Usb)
        .map(|d| (d.udid.clone(), d))
        .collect())
}

async fn battery(device: idevice::usbmuxd::UsbmuxdDevice) -> Result<Map<String, Value>, String> {
    let provider = device.to_provider(UsbmuxdAddr::default(), LABEL);
    let mut relay = DiagnosticsRelayClient::connect(&provider)
        .await
        .map_err(|e| format!("diagnostics_relay: {e}"))?;
    let props = relay
        .ioregistry(None, None, Some("AppleSmartBattery"))
        .await
        .map_err(|e| format!("IORegistry: {e}"))?
        .ok_or("no AppleSmartBattery")?;
    let _ = relay.goodbye().await;
    match serde_json::to_value(plist::Value::Dictionary(props)) {
        Ok(Value::Object(map)) => Ok(map),
        _ => Err("AppleSmartBattery is not a dictionary".into()),
    }
}
