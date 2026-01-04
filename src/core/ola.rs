use super::frame_processor::FrameProcessor;
use crate::core::channels::Mono;
use alloc::collections::VecDeque;
#[cfg(feature = "debug_visualize")]
use alloc::format;
#[cfg(feature = "debug_visualize")]
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::f32::consts::PI;
use num_complex::{Complex32, ComplexFloat};

/// Trait for processors that operate on spectral data (FFT bins).
pub trait SpectralProcessor {
    /// Process a block of complex spectral bins.
    ///
    /// # Arguments
    /// * `bins` - The spectral data.
    /// * `sample_index` - The sample index corresponding to the start of the analysis window.
    fn process_spectral(&mut self, bins: &mut [Complex32], sample_index: u64);

    /// Returns the name of the spectral processor.
    fn name(&self) -> &str {
        #[cfg(feature = "debug_visualize")]
        {
            "SpectralProcessor"
        }
        #[cfg(not(feature = "debug_visualize"))]
        {
            ""
        }
    }
}

/// Helper trait to abstract FFT operations for different sizes.
pub trait FftHelper {
    fn do_fft(&mut self);
    fn do_ifft(&mut self);
}

impl FftHelper for [Complex32; 256] {
    fn do_fft(&mut self) {
        let _ = microfft::complex::cfft_256(self);
    }
    fn do_ifft(&mut self) {
        for x in self.iter_mut() {
            *x = x.conj();
        }
        let _ = microfft::complex::cfft_256(self);
        for x in self.iter_mut() {
            *x = x.conj() / 256.0;
        }
    }
}

impl FftHelper for [Complex32; 512] {
    fn do_fft(&mut self) {
        let _ = microfft::complex::cfft_512(self);
    }
    fn do_ifft(&mut self) {
        for x in self.iter_mut() {
            *x = x.conj();
        }
        let _ = microfft::complex::cfft_512(self);
        for x in self.iter_mut() {
            *x = x.conj() / 512.0;
        }
    }
}

impl FftHelper for [Complex32; 1024] {
    fn do_fft(&mut self) {
        let _ = microfft::complex::cfft_1024(self);
    }
    fn do_ifft(&mut self) {
        for x in self.iter_mut() {
            *x = x.conj();
        }
        let _ = microfft::complex::cfft_1024(self);
        for x in self.iter_mut() {
            *x = x.conj() / 1024.0;
        }
    }
}

impl FftHelper for [Complex32; 2048] {
    fn do_fft(&mut self) {
        let _ = microfft::complex::cfft_2048(self);
    }
    fn do_ifft(&mut self) {
        for x in self.iter_mut() {
            *x = x.conj();
        }
        let _ = microfft::complex::cfft_2048(self);
        for x in self.iter_mut() {
            *x = x.conj() / 2048.0;
        }
    }
}

/// Overlap-Add (OLA) processor for spectral effects.
///
/// Handles windowing, FFT, processing, IFFT, and overlap-add reconstruction.
/// Supports block sizes independent of FFT size.
///
/// This processor operates on Mono signals only
pub struct Ola<P: SpectralProcessor, const N: usize> {
    processor: P,
    window: [f32; N],
    hop_size: usize,

    input_queue: VecDeque<f32>,
    output_queue: VecDeque<f32>,

    fft_buffer: [Complex32; N],
    ola_buffer: Vec<f32>,

    current_sample_index: u64,
}

impl<P: SpectralProcessor, const N: usize> Ola<P, N>
where
    [Complex32; N]: FftHelper,
{
    /// Creates a new OLA processor.
    ///
    /// # Arguments
    /// * `processor` - The spectral processor to apply.
    pub fn with(processor: P) -> Self {
        let mut window = [0.0; N];
        for (i, w) in window.iter_mut().enumerate() {
            let arg = 2.0 * PI * i as f32 / (N - 1) as f32;
            *w = 0.5 * (1.0 - libm::cosf(arg));
        }

        let hop_size = N / 2;
        let output_queue = VecDeque::from(vec![0.0; N]);

        Ola {
            processor,
            window,
            hop_size,
            input_queue: VecDeque::with_capacity(N * 2),
            output_queue,
            fft_buffer: [Complex32::new(0.0, 0.0); N],
            ola_buffer: vec![0.0; N],
            current_sample_index: 0,
        }
    }
}

impl<P: SpectralProcessor, const N: usize> FrameProcessor<Mono> for Ola<P, N>
where
    [Complex32; N]: FftHelper,
{
    fn process(&mut self, buffer: &mut [f32], sample_index: u64) {
        if self.input_queue.is_empty() {
            self.current_sample_index = sample_index;
        }

        for &sample in buffer.iter() {
            self.input_queue.push_back(sample);
        }

        while self.input_queue.len() >= N {
            for i in 0..N {
                self.fft_buffer[i] = Complex32::new(self.input_queue[i] * self.window[i], 0.0);
            }

            self.fft_buffer.do_fft();

            self.processor
                .process_spectral(&mut self.fft_buffer, self.current_sample_index);

            self.fft_buffer.do_ifft();

            for i in 0..N {
                self.ola_buffer[i] += self.fft_buffer[i].re;
            }

            for i in 0..self.hop_size {
                self.output_queue.push_back(self.ola_buffer[i]);
            }

            for i in 0..self.hop_size {
                self.ola_buffer[i] = self.ola_buffer[i + self.hop_size];
                self.ola_buffer[i + self.hop_size] = 0.0;
            }

            self.input_queue.drain(0..self.hop_size);
            self.current_sample_index += self.hop_size as u64;
        }

        for sample in buffer.iter_mut() {
            if let Some(val) = self.output_queue.pop_front() {
                *sample = val;
            } else {
                *sample = 0.0;
            }
        }
    }

    fn set_sample_rate(&mut self, _sample_rate: f32) {}

    fn reset(&mut self) {
        self.input_queue.clear();
        self.output_queue.clear();
        self.output_queue.extend(vec![0.0; N]);
        self.ola_buffer.fill(0.0);
        self.current_sample_index = 0;
    }

    #[cfg(feature = "debug_visualize")]
    fn name(&self) -> &str {
        "Ola (Spectral Wrapper)"
    }

    #[cfg(feature = "debug_visualize")]
    fn visualize(&self, indent: usize) -> String {
        let spaces = " ".repeat(indent);
        format!(
            "{}Ola (FFT Size: {})\n{}  |-- {}\n",
            spaces,
            N,
            spaces,
            self.processor.name()
        )
    }
}
