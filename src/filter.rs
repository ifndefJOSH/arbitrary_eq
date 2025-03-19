use jack::jack_sys::jack_default_audio_sample_t;
use std::f32::consts::{self, PI};

pub type Sample = jack_default_audio_sample_t;

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
pub struct FilterCoefficients<const OrderPlusOne: usize> {
	a: [Sample; OrderPlusOne],
	b: [Sample; OrderPlusOne],
}

// pub struct FilterHistory<const Order: usize

impl<const OrderPlusOne: usize> Default for FilterCoefficients<OrderPlusOne> {
	fn default() -> Self {
		Self {
			a: [0.0; OrderPlusOne],
			b: [0.0; OrderPlusOne],
		}
	}
}

// #[derive(Default)]
pub struct FilterHistory<const Order: usize> {
	x: [Sample; Order],
	y: [Sample; Order],
}

// pub struct FilterHistory<const Order: usize

impl<const Order: usize> Default for FilterHistory<Order> {
	fn default() -> Self {
		Self {
			x: [0.0; Order],
			y: [0.0; Order],
		}
	}
}

pub type SecondOrderCoeffs = FilterCoefficients<3>;
pub type SecondOrderHistory = FilterHistory<2>;

#[derive(Copy, Clone, PartialEq)]
enum FilterType {
	LowPass,
	HighPass,
	BandPass,
}

pub struct LinearFilter {
	/// Filter coefficients
	coeffs: SecondOrderCoeffs,
	/// Sample buffer
	hist: SecondOrderHistory,
	/// Sampling frequency
	fs: f32,
	/// Center frequency (in radians): 2.0 * pi * f0 (where f0 is the provided center frequency
	w0: f32,
	/// The gain in dB of the filter or the resonance (if it's a LPF/HPF)
	gain_or_resonance: f32,
	/// Filter type (bandpass is the constant skirt gain with peak gain of Q)
	filter_type: FilterType,
}

impl LinearFilter {
	pub fn ftype(&self) -> FilterType {
		self.filter_type
	}

	pub fn new_lpf(fs: f32, f0: f32, q: f32) -> Self {
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
			fs: fs,
			w0: w0,
			gain_or_resonance: q,
			filter_type: FilterType::LowPass,
			hist: SecondOrderHistory::default(),
			coeffs: FilterCoefficients {
				a: [a0, a1, a2],
				b: [b0, b1, b2],
			}
		}
	}


	pub fn new_hpf(fs: f32, f0: f32, q: f32) -> Self {
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
			fs: fs,
			w0: w0,
			gain_or_resonance: q,
			filter_type: FilterType::HighPass,
			hist: SecondOrderHistory::default(),
			coeffs: FilterCoefficients {
				a: [a0, a1, a2],
				b: [b0, b1, b2],
			}
		}

	}

	pub fn new_bpf(fs: f32, f0: f32, q: f32) -> Self {
		let w0 = 2.0 * PI * f0;
		let cosw0 = w0.cos();
		let alpha = w0.sin() / (2.0 / q);


		// Compute the filter coefficients
		let b0 = q*alpha;
		let b1 = 0.0;
		let b2 = -q*alpha;
		let a0 = 1.0 + alpha;
		let a1 = -2.0 * cosw0;
		let a2 = 1.0 - alpha;

		Self {
			fs: fs,
			w0: w0,
			gain_or_resonance: q,
			filter_type: FilterType::BandPass,
			hist: SecondOrderHistory::default(),
			coeffs: FilterCoefficients {
				a: [a0, a1, a2],
				b: [b0, b1, b2],
			}
		}
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
		yn
	}

	fn gain_or_q(&self) -> f32 {
	    unimplemented!();
	}

	fn center_freq(&self) -> f32 {
	    self.w0 / (2.0 * PI)
	}
}
