use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

use super::resampler::Resampler;

#[derive(Debug, Clone, Serialize)]
pub struct AudioDevice {
    pub name: String,
    pub is_default: bool,
}

/// Manages audio capture from microphone input devices.
pub struct AudioCaptureManager {
    stream: Option<Stream>,
    is_paused: Arc<AtomicBool>,
}

impl AudioCaptureManager {
    pub fn new() -> Self {
        Self {
            stream: None,
            is_paused: Arc::new(AtomicBool::new(false)),
        }
    }

    /// List available audio input devices.
    pub fn list_devices() -> Result<Vec<AudioDevice>, String> {
        let host = cpal::default_host();
        let default_device = host.default_input_device();
        let default_name = default_device
            .as_ref()
            .and_then(|d| d.name().ok())
            .unwrap_or_default();

        let devices = host
            .input_devices()
            .map_err(|e| format!("Failed to enumerate input devices: {}", e))?;

        let mut result = Vec::new();
        for device in devices {
            if let Ok(name) = device.name() {
                result.push(AudioDevice {
                    is_default: name == default_name,
                    name,
                });
            }
        }
        Ok(result)
    }

    /// Start capturing audio from the specified device (or default).
    /// Sends 16kHz mono f32 audio chunks through the channel.
    pub fn start(
        &mut self,
        device_name: Option<&str>,
        sender: mpsc::UnboundedSender<Vec<f32>>,
    ) -> Result<(), String> {
        let host = cpal::default_host();

        let device = match device_name {
            Some(name) => find_device_by_name(&host, name)?,
            None => host
                .default_input_device()
                .ok_or("No default input device found")?,
        };

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        let sample_format = config.sample_format();

        let resampler = Resampler::new(sample_rate, channels);
        let is_paused = Arc::clone(&self.is_paused);
        is_paused.store(false, Ordering::Relaxed);

        let stream_config: StreamConfig = config.into();

        let err_fn = |err: cpal::StreamError| {
            eprintln!("Audio stream error: {}", err);
        };

        let stream = match sample_format {
            SampleFormat::F32 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if is_paused.load(Ordering::Relaxed) {
                            return;
                        }
                        let resampled = resampler.resample(data);
                        if !resampled.is_empty() {
                            let _ = sender.send(resampled);
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Failed to build input stream: {}", e))?,
            SampleFormat::I16 => {
                let resampler = Resampler::new(sample_rate, channels);
                let is_paused = Arc::new(AtomicBool::new(false));
                device
                    .build_input_stream(
                        &stream_config,
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            if is_paused.load(Ordering::Relaxed) {
                                return;
                            }
                            let float_data: Vec<f32> =
                                data.iter().map(|&s| s as f32 / 32768.0).collect();
                            let resampled = resampler.resample(&float_data);
                            if !resampled.is_empty() {
                                let _ = sender.send(resampled);
                            }
                        },
                        err_fn,
                        None,
                    )
                    .map_err(|e| format!("Failed to build i16 input stream: {}", e))?
            }
            _ => return Err(format!("Unsupported sample format: {:?}", sample_format)),
        };

        stream
            .play()
            .map_err(|e| format!("Failed to start stream: {}", e))?;

        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.stream = None;
    }

    pub fn pause(&self) {
        self.is_paused.store(true, Ordering::Relaxed);
    }

    pub fn resume(&self) {
        self.is_paused.store(false, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::Relaxed)
    }
}

fn find_device_by_name(host: &cpal::Host, name: &str) -> Result<Device, String> {
    let devices = host
        .input_devices()
        .map_err(|e| format!("Failed to enumerate devices: {}", e))?;

    for device in devices {
        if let Ok(device_name) = device.name() {
            if device_name == name {
                return Ok(device);
            }
        }
    }
    Err(format!("Device '{}' not found", name))
}
