use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};
use infinitedsp_core::core::dsp_chain::DspChain;
use infinitedsp_core::core::frame_processor::FrameProcessor;
use std::sync::{Arc, Mutex};

pub trait StereoProcessor: Send {
    fn process(&mut self, left: &mut [f32], right: &mut [f32], sample_index: u64);
}

pub fn init_audio<F>(create_processor: F) -> Result<(cpal::Stream, f32)>
where
    F: FnOnce(f32) -> DspChain,
{
    let host = cpal::default_host();
    let device = host.default_output_device().expect("No output device available");
    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate() as f32;

    let chain = create_processor(sample_rate);
    let processor = Arc::new(Mutex::new(chain));

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => run_mono::<f32>(&device, &config.into(), processor, err_fn)?,
        cpal::SampleFormat::I16 => run_mono::<i16>(&device, &config.into(), processor, err_fn)?,
        cpal::SampleFormat::U16 => run_mono::<u16>(&device, &config.into(), processor, err_fn)?,
        _ => return Err(anyhow::anyhow!("Unsupported sample format")),
    };

    Ok((stream, sample_rate))
}

pub fn init_audio_stereo<F, P>(create_processor: F) -> Result<(cpal::Stream, f32)>
where
    P: StereoProcessor + 'static,
    F: FnOnce(f32) -> P,
{
    let host = cpal::default_host();
    let device = host.default_output_device().expect("No output device available");
    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate() as f32;

    let engine = create_processor(sample_rate);
    let processor = Arc::new(Mutex::new(engine));

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => run_stereo::<f32, P>(&device, &config.into(), processor, err_fn)?,
        cpal::SampleFormat::I16 => run_stereo::<i16, P>(&device, &config.into(), processor, err_fn)?,
        cpal::SampleFormat::U16 => run_stereo::<u16, P>(&device, &config.into(), processor, err_fn)?,
        _ => return Err(anyhow::anyhow!("Unsupported sample format")),
    };

    Ok((stream, sample_rate))
}

pub fn init_audio_interleaved<F>(create_processor: F) -> Result<(cpal::Stream, f32)>
where
    F: FnOnce(f32) -> DspChain,
{
    let host = cpal::default_host();
    let device = host.default_output_device().expect("No output device available");
    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate() as f32;

    let chain = create_processor(sample_rate);
    let processor = Arc::new(Mutex::new(chain));

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => run_interleaved::<f32>(&device, &config.into(), processor, err_fn)?,
        cpal::SampleFormat::I16 => run_interleaved::<i16>(&device, &config.into(), processor, err_fn)?,
        cpal::SampleFormat::U16 => run_interleaved::<u16>(&device, &config.into(), processor, err_fn)?,
        _ => return Err(anyhow::anyhow!("Unsupported sample format")),
    };

    Ok((stream, sample_rate))
}

fn run_mono<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    processor: Arc<Mutex<DspChain>>,
    err_fn: impl Fn(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut process_buffer = vec![0.0; 512];
    let mut sample_clock = 0u64;

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut proc = processor.lock().unwrap();

            let frames = data.len() / channels;
            if process_buffer.len() < frames {
                process_buffer.resize(frames, 0.0);
            }

            let proc_slice = &mut process_buffer[0..frames];

            proc.process(proc_slice, sample_clock);
            sample_clock += frames as u64;

            for (i, frame) in data.chunks_mut(channels).enumerate() {
                let sample = T::from_sample(proc_slice[i]);
                for channel_sample in frame.iter_mut() {
                    *channel_sample = sample;
                }
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn run_stereo<T, P>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    processor: Arc<Mutex<P>>,
    err_fn: impl Fn(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
    P: StereoProcessor + 'static,
{
    let channels = config.channels as usize;
    let mut left_buffer = vec![0.0; 512];
    let mut right_buffer = vec![0.0; 512];
    let mut sample_clock = 0u64;

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut proc = processor.lock().unwrap();

            let frames = data.len() / channels;
            if left_buffer.len() < frames {
                left_buffer.resize(frames, 0.0);
                right_buffer.resize(frames, 0.0);
            }

            let l_slice = &mut left_buffer[0..frames];
            let r_slice = &mut right_buffer[0..frames];

            l_slice.fill(0.0);
            r_slice.fill(0.0);

            proc.process(l_slice, r_slice, sample_clock);
            sample_clock += frames as u64;

            for (i, frame) in data.chunks_mut(channels).enumerate() {
                let l_sample = T::from_sample(l_slice[i]);
                let r_sample = T::from_sample(r_slice[i]);

                if channels >= 2 {
                    frame[0] = l_sample;
                    frame[1] = r_sample;
                } else {
                    frame[0] = T::from_sample((l_slice[i] + r_slice[i]) * 0.5);
                }
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn run_interleaved<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    processor: Arc<Mutex<DspChain>>,
    err_fn: impl Fn(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut process_buffer = vec![0.0; 512];
    let mut sample_clock = 0u64;

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let mut proc = processor.lock().unwrap();

            if process_buffer.len() < data.len() {
                process_buffer.resize(data.len(), 0.0);
            }

            let proc_slice = &mut process_buffer[0..data.len()];

            proc.process(proc_slice, sample_clock);
            sample_clock += (data.len() / channels) as u64;

            for (i, sample) in data.iter_mut().enumerate() {
                *sample = T::from_sample(proc_slice[i]);
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}
