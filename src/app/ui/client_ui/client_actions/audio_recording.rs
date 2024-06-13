//! Records a WAV file (roughly 3 seconds long) using the default input device and config.
//!
//! The input data is recorded to "$APPDATA/szeChat/Client/(base64) - self.send_on_ip/recorded.wav".

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample};
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

struct Opt {
    /// The audio device to use
    device: String,

    /// Use the JACK host
    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd"
        ),
        feature = "jack"
    ))]
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

pub fn audio_recroding(receiver: mpsc::Receiver<bool>, path: Arc<Mutex<PathBuf>>) {
    std::thread::spawn(move || {
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

        // The WAV file we're recording to.

        let spec = wav_spec_from_config(&config);

        let path = path.lock().unwrap().to_string_lossy().to_string().clone();

        let writer = hound::WavWriter::create(path, spec)?;

        let writer = Arc::new(Mutex::new(Some(writer)));

        // A flag to indicate that recording is in progress.

        // Run the input stream on a separate thread.
        let writer_2 = writer.clone();

        let err_fn = move |err| {
            eprintln!("an error occurred on stream: {}", err);
        };

        let stream = match config.sample_format() {
            cpal::SampleFormat::I8 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<i8, i8>(data, &writer_2),
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<i16, i16>(data, &writer_2),
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I32 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<i32, i32>(data, &writer_2),
                err_fn,
                None,
            )?,
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data, _: &_| write_input_data::<f32, f32>(data, &writer_2),
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

        Ok(())
    });
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

type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

fn write_input_data<T, U>(input: &[T], writer: &WavWriterHandle)
where
    T: Sample,
    U: Sample + hound::Sample + FromSample<T>,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                let sample: U = U::from_sample(sample);
                writer.write_sample(sample).ok();
            }
        }
    }
}
