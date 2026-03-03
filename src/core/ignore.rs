use std::fs;
use std::path::Path;

pub struct MoxIgnore {
	patterns: Vec<String>,
}

impl MoxIgnore {
	pub fn load() -> Self {
		let mut patterns = vec![
			".mox".to_string(), // Always ignore the internal folder
			"mox.exe".to_string(),
			"localthumbcache.package".to_string(), // Sims 4 specific default
		];

		if let Ok(content) = fs::read_to_string(".moxignore") {
			for line in content.lines() {
				let trimmed = line.trim();
				if !trimmed.is_empty() && !trimmed.starts_with('#') {
					patterns.push(trimmed.to_string());
				}
			}
		}

		Self { patterns }
	}

	pub fn is_ignored(&self, path: &Path) -> bool {
		let path_str = path.to_str().unwrap_or("");
		self.patterns.iter().any(|pattern| {
			path_str.contains(pattern) || path_str.ends_with(pattern)
		})
	}
}