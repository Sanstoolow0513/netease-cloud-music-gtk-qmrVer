//
// output_device.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use gst::prelude::*;
use gstreamer_play::gst;
use log::{debug, warn};

/// 一个可用的音频输出设备（GStreamer Audio/Sink）
#[derive(Debug, Clone)]
pub struct AudioOutputDevice {
    /// 稳定标识：sink 元素的 "device" 属性值（字符串化）。
    /// 取不到时用显示名兜底（显示名可能随系统语言变化，仅作兜底）。
    pub id: String,
    /// 面向用户的显示名
    pub name: String,
}

/// 在 DeviceMonitor 启动期间执行闭包（设备列表通常只有个位数，启动/停止开销可忽略）
fn with_devices<T>(f: impl FnOnce(&[gst::Device]) -> T) -> T {
    let monitor = gst::DeviceMonitor::new();
    monitor.add_filter(Some("Audio/Sink"), None);
    if let Err(err) = monitor.start() {
        warn!("启动 GStreamer 设备监视器失败: {err:?}");
        return f(&[]);
    }
    let devices: Vec<gst::Device> = monitor.devices().into_iter().collect();
    let result = f(&devices);
    monitor.stop();
    result
}

/// 提取设备稳定 id：create_element 会按设备预配置 sink 元素，
/// 读取其 "device" 属性（wasapi 为 GUID 字符串，osxaudiosink 为 uint）
fn device_id(device: &gst::Device) -> Option<String> {
    let element = device.create_element(None).ok()?;
    element.find_property("device")?;
    let value = element.property_value("device");
    if let Ok(s) = value.get::<String>() {
        if !s.is_empty() {
            return Some(s);
        }
    }
    if let Ok(u) = value.get::<u32>() {
        return Some(u.to_string());
    }
    None
}

/// 枚举系统音频输出设备（按 id 去重：同一设备可能被多个 provider 枚举，如 wasapi 与 wasapi2）
pub fn enumerate() -> Vec<AudioOutputDevice> {
    with_devices(|devices| {
        let mut result: Vec<AudioOutputDevice> = Vec::new();
        for device in devices {
            let name = device.display_name().to_string();
            let id = device_id(device).unwrap_or_else(|| name.clone());
            if result.iter().any(|d| d.id == id) {
                continue;
            }
            result.push(AudioOutputDevice { id, name });
        }
        debug!("枚举到音频输出设备: {result:?}");
        result
    })
}

/// 按 id 查找设备并创建对应的 sink 元素；找不到（设备已拔出）返回 None
pub fn find_sink(id: &str) -> Option<gst::Element> {
    with_devices(|devices| {
        devices.iter().find_map(|device| {
            let matched =
                device_id(device).map(|i| i == id).unwrap_or(false) || device.display_name() == id;
            if matched {
                device.create_element(None).ok()
            } else {
                None
            }
        })
    })
}
