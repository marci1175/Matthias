//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$APPDATA/szeChat/Client/(base64) - self.send_on_ip/recorded.wav".

use anyhow::Error;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, SupportedStreamConfig};
use hound::WavWriter;
use opus::Encoder;
use std::collections::VecDeque;
use std::f32;
use std::io::{BufWriter, Cursor};
use std::sync::mpsc::{self};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::app::ui::client::VOIP_PACKET_BUFFER_LENGHT_MS;

pub const SAMPLE_RATE: usize = 48000;
pub const STEREO_PACKET_BUFFER_LENGHT: usize =
    SAMPLE_RATE * 2 * VOIP_PACKET_BUFFER_LENGHT_MS / 1000;

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
/// The amplification precentage is the precent the microphone's volume should be present, this is later turned into a multiplier
pub fn record_audio_for_set_duration(
    dur: Duration,
    amplification_precentage: f32,
) -> anyhow::Result<Vec<f32>> {
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

    let wav_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));

    let wav_buffer_clone = wav_buffer.clone();

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let stream = device.build_input_stream(
        &config.into(),
        move |data, _: &_| {
            write_input_data::<f32>(
                data,
                wav_buffer.clone(),
                amplification_precentage / 100.,
            )
        },
        err_fn,
        None,
    )?;

    stream.play()?;

    std::thread::sleep(dur);

    let recording = wav_buffer_clone.lock().unwrap().clone();

    Ok(recording)
}

/// This function returns a handle to a `queue` of bytes (```Arc<Mutex<VecDeque<u8>>>```), while spawning a thread which constantly writes the incoming audio into the buffer
/// This  `queue` or `buffer` gets updated from left to right, the new element always pushes back all the elements behind it, if the value's index reaches ```idx > queue_lenght```, it gets dropped.
pub fn record_audio_with_interrupt(
    interrupt: CancellationToken,
    amplification_precentage: f32,
    buffer_handle: Arc<Mutex<VecDeque<f32>>>,
) -> anyhow::Result<Arc<Mutex<VecDeque<f32>>>> {
    let wav_buffer_clone = buffer_handle.clone();

    let _: JoinHandle<anyhow::Result<()>> = std::thread::spawn(move || {
        //Create scope, so it will show the compiler that the ```stream``` will NOT be used after the await
        let (device, config) = get_recording_device()?;

        let err_fn = move |err| {
            eprintln!("An error occurred on stream: {}", err);
        };

        let stream = device.build_input_stream(
            &config.into(),
            move |data, _: &_| {
                write_input_data_to_buffer_with_set_len::<f32>(
                    data,
                    buffer_handle.clone(),
                    amplification_precentage / 100.,
                )
            },
            err_fn,
            None,
        )?;

        stream.play()?;

        //Wait for interrupt
        while !interrupt.is_cancelled() {}

        //End thread
        Ok(())
    });

    Ok(wav_buffer_clone)
}

/// This function fetches the audio recording device, returning a result of the Device and Config handle
fn get_recording_device() -> Result<(Device, SupportedStreamConfig), Error> {
    let opt = Opt::default();
    let host = cpal::default_host();
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
    Ok((device, config))
}

/// This function records audio on a different thread, until the reciver recives something, then the recorded buffer is returned
/// The amplification precentage is the precent the microphone's volume should be present, this is later turned into a multiplier
pub fn audio_recording_with_recv(
    receiver: mpsc::Receiver<bool>,
    amplification_precentage: f32,
) -> anyhow::Result<Vec<f32>> {
    let (device, config) = get_recording_device()?;

    let wav_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));

    let wav_buffer_clone = wav_buffer.clone();

    let err_fn = move |err| {
        eprintln!("an error occurred on stream: {}", err);
    };

    let stream = device.build_input_stream(
        &config.into(),
        move |data, _: &_| {
            write_input_data::<f32>(
                data,
                wav_buffer.clone(),
                amplification_precentage / 100.,
            )
        },
        err_fn,
        None,
    )?;

    stream.play()?;

    //Block until further notice by user
    receiver.recv()?;

    let recoreded_bytes = wav_buffer_clone
        .clone()
        .lock()
        .map_err(|err| Error::msg(err.to_string()))?
        .clone();

    Ok(recoreded_bytes)
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

/// This function creates a wav foramtted audio file, containing the samples provided to this function
pub fn create_wav_file(samples: Vec<f32>) -> Vec<u8> {
    let writer = Arc::new(Mutex::new(Vec::new()));

    let (_, config) = get_recording_device().unwrap();

    let spec = wav_spec_from_config(&config);

    let writer_lock = &mut *writer.lock().unwrap();

    let mut buf = Cursor::new(writer_lock);

    let mut wav_buffer = WavWriter::new(BufWriter::new(&mut buf), spec).unwrap();

    for sample in samples {
        wav_buffer.write_sample(sample).unwrap();
    }

    drop(wav_buffer);

    buf.into_inner().to_vec()
}

/// This function creates a wav foramtted audio file, containing the samples provided to this function
/// This function doesnt work properly
pub fn create_opus_file(mut samples: Vec<f32>) -> Vec<u8> {
    let mut opus_encoder = Encoder::new(
        SAMPLE_RATE as u32,
        opus::Channels::Stereo,
        opus::Application::Voip,
    )
    .unwrap();

    samples.resize(STEREO_PACKET_BUFFER_LENGHT, 0.);

    

    opus_encoder
        .encode_vec_float(&samples, 512)
        .inspect_err(|err| {
            dbg!(err.description());
        })
        .unwrap()
}

/// This function writes the multiplied (by the ```amplification_multiplier```) samples to the ```writer```
fn write_input_data<T>(input: &[T], writer: Arc<Mutex<Vec<f32>>>, amplification_multiplier: f32)
where
    T: num_traits::cast::ToPrimitive + Sample + std::fmt::Debug,
{
    let mut inp_vec = input
        .iter()
        .map(|num| num.to_f32().unwrap() * amplification_multiplier)
        .collect::<Vec<f32>>();

    writer.lock().unwrap().append(&mut inp_vec);
}

/// This function writes the multiplied (by the ```amplification_multiplier```) samples to the ```buffer_handle```, and keeps the buffer the lenght of ```len```
/// It will notify the Condvar if the buffer_handle's len reaches ```len```
fn write_input_data_to_buffer_with_set_len<T>(
    input: &[T],
    buffer_handle: Arc<Mutex<VecDeque<f32>>>,
    amplification_multiplier: f32,
) where
    T: num_traits::cast::ToPrimitive + Sample + std::fmt::Debug,
{
    let mut buffer_handle = buffer_handle.lock().unwrap();

    for sample in input.iter() {
        let sample_as_f32 = sample.to_f32().unwrap() * amplification_multiplier;

        buffer_handle.push_back(sample_as_f32);
    }
}
