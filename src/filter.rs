use jack::jack_sys::jack_default_audio_sample_t;
use std::f32::consts::PI;

pub type Sample = jack_default_audio_sample_t; // f32

pub trait Filter: Send {
	/// Performs the filter on a single sample, returns the output sample
	fn filter(&mut self, xn: Sample) -> Sample;
	/// Gets the center frequency
	fn center_freq(&self) -> f32;
	/// Gets the gain or resonance depending on the filter type
	fn gain_or_q(&self) -> f32;
	/// Filters an entire frame of audio
	fn filter_frame(&mut self, frame: &[Sample], output_frame: &mut [Sample]) {
		output_frame.iter_mut().zip(frame.iter()).for_each(|(yn, xn)| {
			*yn = self.filter(*xn);
		})
	}
}

// #[derive(Default)]
pub struct FilterCoefficients<const ORDER_PLUS_ONE: usize> {
	a: [Sample; ORDER_PLUS_ONE],
	b: [Sample; ORDER_PLUS_ONE],
}

// pub struct FilterHistory<const Order: usize

impl<const ORDER_PLUS_ONE: usize> Default for FilterCoefficients<ORDER_PLUS_ONE> {
	fn default() -> Self {
		Self {
			a: [0.0; ORDER_PLUS_ONE],
			b: [0.0; ORDER_PLUS_ONE],
		}
	}
}

// #[derive(Default)]
pub struct FilterHistory<const ORDER: usize> {
	x: [Sample; ORDER],
	y: [Sample; ORDER],
}

// pub struct FilterHistory<const ORDER: usize

impl<const ORDER: usize> Default for FilterHistory<ORDER> {
	fn default() -> Self {
		Self {
			x: [0.0; ORDER],
			y: [0.0; ORDER],
		}
	}
}

pub type SecondOrderCoeffs = FilterCoefficients<3>;
pub type SecondOrderHistory = FilterHistory<2>;

impl SecondOrderCoeffs {

		pub fn new_lpf(f0: f32, q: f32) -> Self {
		let w0 = 2.0 * PI * f0;
		let cosw0 = w0.cos();
		let alpha = w0.sin() / (2.0 / q);

		// Compute the filter coefficients
		let b0 = (1.0 - cosw0) / 2.0;
        let b1 =  1.0 - cosw0;
        let b2 = (1.0 - cosw0) / 2.0;
        let a0 =  1.0 + alpha;
        let a1 = -2.0 * cosw0;
        let a2 =  1.0 - alpha;
		Self {
			a: [a0, a1, a2],
			b: [b0, b1, b2],
		}

	}


	pub fn new_hpf(f0: f32, q: f32) -> Self {
		let w0 = 2.0 * PI * f0;
		let cosw0 = w0.cos();
		let alpha = w0.sin() / (2.0 / q);

		// Compute the filter coefficients
		let b0 = (1.0 + cosw0) / 2.0;
		let b1 = -(1.0 + cosw0);
		let b2 = (1.0 + cosw0) / 2.0;
		let a0 =  1.0 + alpha;
		let a1 = -2.0 * cosw0;
		let a2 = 1.0 - alpha;
		Self {
			a: [a0, a1, a2],
			b: [b0, b1, b2],
		}
	}

	pub fn new_bpf(f0: f32, q: f32) -> Self {
		let w0 = 2.0 * PI * f0;
		let cosw0 = w0.cos();
		let alpha = w0.sin() / (2.0 / q);

		// Compute the filter coefficients
		let b0 = q * alpha;
		let b1 = 0.0;
		let b2 = -q * alpha;
		let a0 = 1.0 + alpha;
		let a1 = -2.0 * cosw0;
		let a2 = 1.0 - alpha;
		Self {
			a: [a0, a1, a2],
			b: [b0, b1, b2],
		}
	}
}

#[derive(Copy, Clone, PartialEq)]
pub enum FilterType {
	LowPass,
	HighPass,
	BandPass,
}

pub struct LinearFilter {
	/// Whether or not the filter is enabled
	enabled: bool,
	/// Filter coefficients
	coeffs: SecondOrderCoeffs,
	/// Sample buffer
	hist: SecondOrderHistory,
	/// Sampling frequency
	fs: f32,
	/// Center frequency (in radians): 2.0 * pi * f0 (where f0 is the provided center frequency
	f0: f32,
	/// The gain in dB of the filter or the resonance (if it's a LPF/HPF)
	gain_or_resonance: f32,
	/// Filter type (bandpass is the constant skirt gain with peak gain of Q)
	filter_type: FilterType,
}

impl LinearFilter {
	pub fn ftype(&self) -> FilterType {
		self.filter_type
	}

	pub fn new(fs: f32, f0: f32, q: f32, filter_type: FilterType) -> Self {
		match filter_type {
			FilterType::LowPass => Self::new_lpf(fs, f0, q),
			FilterType::HighPass => Self::new_hpf(fs, f0, q),
			FilterType::BandPass => Self::new_bpf(fs, f0, q),
		}
	}

	pub fn new_lpf(fs: f32, f0: f32, q: f32) -> Self {
		Self {
			enabled: true,
			fs,
			f0,
			gain_or_resonance: q,
			filter_type: FilterType::LowPass,
			hist: SecondOrderHistory::default(),
			coeffs: FilterCoefficients::new_lpf(f0, q),
		}
	}


	pub fn new_hpf(fs: f32, f0: f32, q: f32) -> Self {
		Self {
			enabled: true,
			fs,
			f0,
			gain_or_resonance: q,
			filter_type: FilterType::HighPass,
			hist: SecondOrderHistory::default(),
			coeffs: FilterCoefficients::new_hpf(f0, q),
		}
	}

	pub fn new_bpf(fs: f32, f0: f32, q: f32) -> Self {
		Self {
			enabled: true,
			fs,
			f0,
			gain_or_resonance: q,
			filter_type: FilterType::BandPass,
			hist: SecondOrderHistory::default(),
			coeffs: FilterCoefficients::new_bpf(f0, q),
		}
	}

	pub fn recalculate_coeffs(&mut self, f0: f32, q: f32) {
		self.f0 = f0;
		self.gain_or_resonance = q;
		self.coeffs = match self.filter_type {
			FilterType::LowPass => FilterCoefficients::new_lpf(f0, q),
			FilterType::HighPass => FilterCoefficients::new_hpf(f0, q),
			FilterType::BandPass => FilterCoefficients::new_bpf(f0, q),
		};
	}

	pub fn update_f0(&mut self, f0: f32) {
		self.recalculate_coeffs(f0, self.gain_or_resonance);
	}

	pub fn update_gain_or_q(&mut self, gain_or_q: f32) {
		self.recalculate_coeffs(self.f0, gain_or_q);
	}
}

impl Filter for LinearFilter {
	fn filter(&mut self, xn: Sample) -> Sample {
		let a = self.coeffs.a;
		let b = self.coeffs.b;
		let mut x = self.hist.x;
		let mut y = self.hist.y;
		let yn = (b[0]/a[0]) * xn + (b[1] / a[0])*x[0] + (b[2]/a[0]) * x[1]
			- (a[1]/a[0]) * y[0] - (a[2]/a[0])*y[1];
		// update the history
		(x[0], x[1]) = (xn, x[0]);
		(y[0], y[1]) = (yn, y[0]);
		// Return the new result
		yn
	}

	fn gain_or_q(&self) -> f32 {
		self.gain_or_resonance
	}

	fn center_freq(&self) -> f32 {
	    self.f0
	}
}

pub struct Equalizer {
	filters: Vec<LinearFilter>,
}

impl Equalizer {
	fn add_filter(&mut self, fs: f32, f0: f32, q: f32, filter_type: FilterType) {
		self.filters.push(LinearFilter::new(fs, f0, q, filter_type));
	}

	fn new(bands: usize, fs: f32) -> Self {
		let num_filters = bands + 2; // Because we want lowpass and highpass filters.
		Self {
			filters: (1..=num_filters).map(|i| {
				let f0 = fs * (i as f32 / num_filters as f32);
				match i {
					// lowpass filter at the bottom
					1 => LinearFilter::new_lpf(fs, f0, 1.0),
					// Highpass filter at the top
					num_filters => LinearFilter::new_hpf(fs, f0, 1.0),
					// Bandpass everywhere else
					_ => LinearFilter::new_bpf(fs, f0, 1.0),
				}
			}).collect::<Vec<_>>()
		}
	}
}

impl Filter for Equalizer {
	fn filter(&mut self, xn: Sample) -> Sample {
	    self.filters.iter_mut().fold(xn, |xint, filter| {
			filter.filter(xint)
		})
	}

	fn gain_or_q(&self) -> f32 {
	    unimplemented!();
	}

	fn center_freq(&self) -> f32 {
	    unimplemented!();
	}
}
