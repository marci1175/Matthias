//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$APPDATA/szeChat/Client/(base64) - self.send_on_ip/recorded.wav".

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, FromSample, Sample, SupportedStreamConfig};
use hound::WavWriter;
use pipe::PipeWriter;
use std::f32;
use std::fs::File;
use std::io::{BufWriter, Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct Opt {
    /// The audio device to use
    device: String,

    /// Use the JACK host
    #[cfg(all(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd"
    ),))]
    #[arg(short, long)]
    #[allow(dead_code)]
    jack: bool,
}
impl Default for Opt {
    fn default() -> Self {
        Self {
            device: "default".into(),
        }
    }
}

/// This function records audio for the passed in duration, then it reutrns the recorded bytes wrapped in a result
pub fn record_audio_for_set_duration(dur: Duration) -> anyhow::Result<Vec<f32>> {
    let opt = Opt::default();

    let host = cpal::default_host();

    // Set up the input device and stream with the default input config.
    let device = if opt.device == "default" {
        host.default_input_device()
    } else {
        host.input_devices()?
            .find(|x| x.name().map(|y| y == opt.device).unwrap_or(false))
    }
    .expect("failed to find input device");

    let config = device
        .default_input_config()
        .expect("Failed to get default input config");

    let mut wav_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));

    let wav_buffer_clone = wav_buffer.clone();

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::I8 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i8, i8>(data, &mut wav_buffer),
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i16, i16>(data, &mut wav_buffer),
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i32, i32>(data, &mut wav_buffer),
            err_fn,
            None,
        )?,
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<f32, f32>(data, &mut wav_buffer),
            err_fn,
            None,
        )?,
        sample_format => {
            return Err(anyhow::Error::msg(format!(
                "Unsupported sample format '{sample_format}'"
            )))
        }
    };

    stream.play()?;

    std::thread::sleep(dur);

    return Ok(wav_buffer_clone.lock().unwrap().clone());
}

fn get_config_and_device() -> anyhow::Result<(SupportedStreamConfig, Device)> {
    
    let opt = Opt::default();

    let host = cpal::default_host();

    // Set up the input device and stream with the default input config.
    let device = if opt.device == "default" {
        host.default_input_device()
    } else {
        host.input_devices()?
            .find(|x| x.name().map(|y| y == opt.device).unwrap_or(false))
    }
    .expect("failed to find input device");

    let config = device
        .default_input_config()
        .expect("Failed to get default input config");

    Ok((config, device))
}

/// This function records audio on a different thread, until the reciver recives something, then the recorded buffer is returned
pub fn audio_recording_with_recv(
    receiver: mpsc::Receiver<bool>,
) -> anyhow::Result<Vec<f32>> {
    let (config, device) = get_config_and_device()?;

    let mut wav_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));

    let wav_buffer_clone = wav_buffer.clone();

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::I8 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i8, i8>(data, &mut wav_buffer),
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i16, i16>(data, &mut wav_buffer),
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i32, i32>(data, &mut wav_buffer),
            err_fn,
            None,
        )?,
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<f32, f32>(data, &mut wav_buffer),
            err_fn,
            None,
        )?,
        sample_format => {
            return Err(anyhow::Error::msg(format!(
                "Unsupported sample format '{sample_format}'"
            )))
        }
    };

    stream.play()?;

    //Block until further notice by user
    receiver.recv()?;

    Ok(wav_buffer_clone.clone().lock().unwrap().clone())
}

fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
    if format.is_float() {
        hound::SampleFormat::Float
    } else {
        hound::SampleFormat::Int
    }
}

fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
    hound::WavSpec {
        channels: config.channels() as _,
        sample_rate: config.sample_rate().0 as _,
        bits_per_sample: (config.sample_format().sample_size() * 8) as _,
        sample_format: sample_format(config.sample_format()),
    }
}

pub fn create_playbackable_audio(recording: Vec<f32>) -> Vec<u8> {
    let writer = Arc::new(Mutex::new(Vec::new()));
    let (config, device) = get_config_and_device().unwrap();

    let spec = wav_spec_from_config(&config);

    let mut buf = Cursor::new(writer.lock().unwrap().clone());

    let mut wav_buffer = WavWriter::new(BufWriter::new(&mut buf), spec).unwrap();

    for sample in recording {
        wav_buffer.write_sample(sample);
    }

    drop(wav_buffer);

    buf.into_inner()
}

type WavWriterHandle = Arc<Mutex<Vec<f32>>>;

fn write_input_data<T, U>(input: &[T], writer: &mut WavWriterHandle)
where
    T: num_traits::cast::ToPrimitive + Sample + std::fmt::Debug,
{
    let inp_vec = input
        .iter()
        .map(|num| num.to_f32().unwrap())
        .collect::<Vec<f32>>();

    writer.lock().unwrap().clone_from(&inp_vec);
}
