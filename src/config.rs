use std::collections::HashMap;
use std::default::Default;
use std::fs::OpenOptions;
use std::io::Read;
use toml;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct WatchdogCfg {
	#[serde(skip)]
	pub dir: String,
	#[serde(default)]
	pub auto_restart: bool,
	#[serde(default)]
	pub test_on_redeploy: bool,
	#[serde(default)]
	pub env: HashMap<String, String>,
}

macro_rules! try_default {
	($expr:expr) => (if let Ok(inner) = $expr { inner } else { return WatchdogCfg::default(); })
}

impl WatchdogCfg {
	pub fn load(dir: &str) -> WatchdogCfg {
		let mut cfg = WatchdogCfg::load_impl(dir);

		cfg.dir = dir.to_string();

		cfg
	}

	fn load_impl(dir: &str) -> WatchdogCfg {
		let mut file = try_default!(OpenOptions::new().read(true).open(&format!("{}/Watchdog.toml", dir)));

		let mut buf = String::new();

		if file.read_to_string(&mut buf).is_ok() {
			try_default!(toml::from_str(&buf))
		} else {
			WatchdogCfg::default()
		}
	}
}
