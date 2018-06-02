extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;

use config::WatchdogCfg;
use std::fs;
use std::thread;

mod config;
mod processes;

fn main() {
	for cfg in scrape_watchdog_configs() {
		let _ = thread::Builder::new().name(format!("{}", cfg.dir)).spawn(move || {
			let mut cfg = cfg;
			loop {
				cfg = if let Ok(wdc) = processes::WatchedChild::spawn(cfg) {
					wdc.bind(|_, _| {})
				} else { break; };
			}
		});
	}
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