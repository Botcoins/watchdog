use config::WatchdogCfg;
use std::fs;
use std::process::{Child, Command};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;

pub struct WatchedChild {
	cfg: WatchdogCfg,
	exe_path: String,
	child: Child,
}

impl WatchedChild {
	pub fn spawn(cfg: WatchdogCfg, rebuild: bool) -> Result<WatchedChild, &'static str> {
		let exe_path = WatchedChild::gen_exe(&cfg, rebuild)?;
		let child = WatchedChild::spawn_child(&exe_path, &cfg);

		Ok(WatchedChild { cfg, exe_path, child })
	}

	fn spawn_child(exe_path: &str, cfg: &WatchdogCfg) -> Child {
		let mut cmd = Command::new(&exe_path);

		for (k, v) in &cfg.env {
			cmd.env(k, v);
		}

		cmd.current_dir(&cfg.dir);

		cmd.spawn().expect("failed to spawn child")
	}

	pub fn gen_exe(cfg: &WatchdogCfg, rebuild: bool) -> Result<String, &'static str> {
		if rebuild {
			let _ = Command::new("git")
				.arg("pull")
				.current_dir(&cfg.dir)
				.spawn()
				.expect("failed to pull updates with git")
				.wait();

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

		WatchedChild::cleanup(&cfg);

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

		Ok(exe_path)
	}

	pub fn autorestart(&mut self, rx: Receiver<bool>) {
		loop {
			if let Ok(status) = self.child.try_wait() { break status; } else if rx.try_recv().is_ok() {
				self.child = WatchedChild::spawn_child(&self.exe_path, &self.cfg);
			} else {
				thread::sleep(Duration::from_secs(5));
				continue;
			}
		};
	}

	pub fn cleanup(cfg: &WatchdogCfg) {
		if let Ok(read) = fs::read_dir("exe/") {
			for entry in read {
				if let Ok(entry) = entry {
					if entry.file_name().to_str().unwrap().starts_with(&cfg.dir) {
						let _ = fs::remove_file(entry.path());
					}
				}
			}
		}
	}
}