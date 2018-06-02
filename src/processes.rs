use config::WatchdogCfg;
use std::fs;
use std::process::{Child, Command, ExitStatus};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;

pub struct WatchedChild {
	cfg: WatchdogCfg,
	child: Child,
}

impl WatchedChild {
	pub fn spawn(cfg: WatchdogCfg, rebuild: bool) -> Result<WatchedChild, &'static str> {
		let child = WatchedChild::spawn_impl(&cfg, rebuild)?;
		Ok(WatchedChild { cfg, child })
	}

	fn spawn_impl(cfg: &WatchdogCfg, rebuild: bool) -> Result<Child, &'static str> {
		if rebuild {
			if cfg.test_on_redeploy {
				if 0 != Command::new("cargo")
					.arg("test")
					.current_dir(&cfg.dir)
					.spawn()
					.expect("failed to test with cargo")
					.wait()
					.unwrap()
					.code()
					.unwrap_or(-1) {
					return Err("Test for build failed...");
				}
			}

			let exit_code = Command::new("cargo")
				.arg("build")
				.current_dir(&cfg.dir)
				.spawn()
				.expect("failed to build with cargo")
				.wait()
				.unwrap()
				.code()
				.unwrap_or(-1);

			if exit_code == 0 {
				return Err("Cannot build executable");
			}
		}

		let origin = format!("{}/target/debug/{}", cfg.dir, cfg.dir);

		let mut exe_path = String::new();
		for discrim in 0..8 {
			let dest = format!("exe/{}-{}", cfg.dir, discrim);
			if fs::copy(&origin, &dest).is_ok() {
				exe_path = dest;
				break;
			}
		}

		if exe_path.len() == 0 {
			return Err("Failed to copy process");
		}

		let mut cmd = Command::new(&exe_path);

		for (k, v) in &cfg.env {
			cmd.env(k, v);
		}

		cmd.current_dir(&cfg.dir);

		Ok(cmd.spawn().expect("failed to spawn child"))
	}

	pub fn bind<F>(mut self, rx: Receiver<bool>, shutdown: F) -> WatchdogCfg where F: FnOnce(Option<ExitStatus>, &WatchedChild) + 'static {
		let status = loop {
			if let Ok(status) = self.child.try_wait() { break status; } else if rx.try_recv().is_ok() {
				return self.cfg;
			} else {
				thread::sleep(Duration::from_secs(5));
				continue;
			}
		};

		(shutdown)(status, &self);

		self.cleanup();
		self.cfg
	}

	pub fn cleanup(&self) {
		if let Ok(read) = fs::read_dir("exe/") {
			for entry in read {
				if let Ok(entry) = entry {
					if entry.file_name().to_str().unwrap().starts_with(&self.cfg.dir) {
						let _ = fs::remove_file(entry.path());
					}
				}
			}
		}
	}
}