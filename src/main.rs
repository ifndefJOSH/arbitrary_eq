use filter::{Filter, LinearFilter};
use std::{process::exit, sync::{Arc, Mutex}};

pub mod filter;

fn main() {
	let (client, _status) = jack::Client::new("filter", jack::ClientOptions::default()).unwrap();
	let filter = Arc::new(Mutex::new(LinearFilter::new_lpf(client.sample_rate() as f32, 400.0, 5.0)));
	let process_callback = create_callback(&client, filter);
	let process = jack::contrib::ClosureProcessHandler::new(process_callback);
	let active_client = client.activate_async((), process).unwrap();
	loop {}

}

fn create_callback(client: &jack::Client, filter: Arc<Mutex<dyn Filter>>) -> impl FnMut(&jack::Client, &jack::ProcessScope) -> jack::Control {
	let unlocked_filter = filter.lock().unwrap();
	let in_port = client.register_port("Input", jack::AudioIn::default()).unwrap();
	let mut out_port = client.register_port("Output", jack::AudioOut::default()).unwrap();
	let process_callback = {
		let filter = Arc::clone(&filter);
		move |_: &jack::Client, ps: & jack::ProcessScope| -> jack::Control {
			let in_slice = in_port.as_slice(ps);
			let out_slice = out_port.as_mut_slice(ps);
			if let Ok(mut owned_filter) = filter.lock() {
				owned_filter.filter_frame(in_slice, out_slice);
			} else {
				eprintln!("Could not gain access to mutex!");
			}
			jack::Control::Continue
		}
	};
	process_callback
}
