//! Server-certificate trust. Mumcord pins fingerprints; a headless bot does the same with a small
//! file per server: the first connection stores the fingerprint (trust on first use), later
//! connections must see the same one. A changed fingerprint is refused and logged, because the bot
//! cannot ask anyone - a person has to look at it.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use mumcord_protocol::ServerCert;

pub struct Pins {
    dir: PathBuf,
}

impl Pins {
    pub fn new(dir: PathBuf) -> Pins {
        Pins {
            dir: dir.join("pins"),
        }
    }

    /// `true` when the certificate may be trusted. A pin file is named after `host:port`.
    pub fn check(&self, server: &str, cert: &ServerCert) -> Result<bool> {
        let name: String = server
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let path = self.dir.join(name);
        if !path.exists() {
            fs::create_dir_all(&self.dir)
                .with_context(|| format!("creating {}", self.dir.display()))?;
            fs::write(&path, format!("{server} {}\n", cert.sha256_hex))
                .with_context(|| format!("writing {}", path.display()))?;
            tracing::info!(
                "pinned the server certificate of {server}: {} (trust on first use)",
                cert.sha256_hex
            );
            return Ok(true);
        }
        let pinned =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let fingerprint = pinned.split_whitespace().nth(1).unwrap_or("");
        if fingerprint == cert.sha256_hex {
            return Ok(true);
        }
        bail!(
            "the server certificate of {server} changed: pinned {fingerprint}, seen {}; \
             check the server and delete {} to re-pin",
            cert.sha256_hex,
            path.display()
        );
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}
