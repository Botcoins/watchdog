extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;

use config::WatchdogCfg;
use std::fs;
use std::io;
use std::process::exit;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::thread;

mod config;
mod processes;

fn main() {
	let mut handles = start_children(true);
	let stdin = io::stdin();

	loop {
		let mut buf = String::new();
		let _ = stdin.read_line(&mut buf);

		let cmd: &str = &buf;

		match cmd {
			"exit" => {
				exit(0)
			}
			"rebuild" => {
				let new_handles = start_children(true);

				for handle in handles.lock().unwrap().iter() {
					let _ = handle.send(true);
				}

				handles = new_handles;
				for handle in handles.lock().unwrap().iter() {
					let _ = handle.send(true);
				}

				println!("Reloaded configs and re-launched all child processes.")
			}
			"reload" => {
				{
					for handle in handles.lock().unwrap().iter() {
						let _ = handle.send(true);
					}
				}

				handles = start_children(false);

				println!("Reloaded configs and re-launched all child processes.")
			}
			_ => println!("Unknown command: Try \"exit\" or \"reload\""),
		};
	}
}

fn start_children(rebuild: bool) -> Arc<Mutex<Vec<Sender<bool>>>> {
	let handles = Arc::new(Mutex::new(vec![]));

	for cfg in scrape_watchdog_configs() {
		let mut handles = Arc::clone(&handles);

		let _ = thread::Builder::new().name(format!("{}", cfg.dir)).spawn(move || {
			if rebuild {
				let _ = processes::WatchedChild::gen_exe(&cfg, true);
			}

			if let Ok(mut wdc) = processes::WatchedChild::spawn(cfg.clone(), false) {
				let (tx, rx) = mpsc::channel();
				handles.lock().unwrap().push(tx);

				if cfg.auto_restart {
					wdc.autorestart(rx);
				}
			}
		}).expect("failed to spawn threads for children");
	}

	handles
}

fn scrape_watchdog_configs() -> Vec<WatchdogCfg> {
	let mut buf = vec![];
	if let Ok(dir) = fs::read_dir("") {
		for entry in dir {
			if let Ok(entry) = entry {
				if entry.path().is_dir() {
					let name_osstr = entry.file_name();
					let name = name_osstr.to_str().unwrap();
					if name != "exe" {
						buf.push(WatchdogCfg::load(name));
					}
				}
			}
		}
	}

	buf
}