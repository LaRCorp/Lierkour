// Lierkour — Copyright (C) 2026 LaRevole (LaRCorp) — GPL-3.0-only

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

pub const LIERKOUR_EXT_PORT: u16 = 8765;
pub const LIERTAGS_PORT: u16 = 8766;
pub const LIERKOUR_SYNC_PORT: u16 = 8767;
pub const LIERSYNC_BRIDGE_PORT: u16 = 8768;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemAppStatus {
    pub id: String,
    pub name: String,
    pub port: u16,
    pub online: bool,
    pub version: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LierSyncSendResult {
    pub success: bool,
    pub target: Option<String>,
    pub sent_files: usize,
    pub bytes: u64,
}

fn port_reachable(port: u16) -> bool {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    TcpStream::connect_timeout(&addr, Duration::from_millis(250)).is_ok()
}

pub fn probe_ecosystem() -> Vec<EcosystemAppStatus> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_millis(700)))
        .build()
        .into();
    let specs = [
        (
            "lierkour",
            "Lierkour (Ext)",
            LIERKOUR_EXT_PORT,
            &["/api/lier/info", "/api/health"][..],
        ),
        (
            "lierkour_sync",
            "Lierkour Sync",
            LIERKOUR_SYNC_PORT,
            &["/api/lier/info", "/api/v1/sync/ping"][..],
        ),
        (
            "liertags",
            "LierTags",
            LIERTAGS_PORT,
            &["/api/lier/info", "/api/status"][..],
        ),
        (
            "liersync",
            "LierSync Bridge",
            LIERSYNC_BRIDGE_PORT,
            &["/api/lier/info"][..],
        ),
    ];

    let mut out = Vec::with_capacity(specs.len());
    for (id, name, port, paths) in specs {
        let mut status = EcosystemAppStatus {
            id: id.to_string(),
            name: name.to_string(),
            port,
            online: false,
            version: None,
            detail: "Offline".to_string(),
        };
        if port_reachable(port) {
            status.online = true;
            status.detail = "Online".to_string();
            for p in paths {
                let url = format!("http://127.0.0.1:{port}{p}");
                if let Ok(mut resp) = agent.get(&url).call()
                    && let Ok(val) = resp.body_mut().read_json::<Value>()
                {
                    if let Some(v) = val.get("version").and_then(|v| v.as_str()) {
                        status.version = Some(v.to_string());
                        status.detail = format!("v{v}");
                    }
                    break;
                }
            }
        }
        out.push(status);
    }
    out
}

pub fn send_paths_to_liersync(paths: &[String]) -> Result<LierSyncSendResult> {
    let url = format!("http://127.0.0.1:{LIERSYNC_BRIDGE_PORT}/api/ecosystem/send-files");
    let body = json!({
        "paths": paths,
        "source": "Lierkour"
    });
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into();

    let mut resp = agent
        .post(&url)
        .send_json(&body)
        .map_err(|e| anyhow::anyhow!("LierSync недоступен на :{LIERSYNC_BRIDGE_PORT} — {e}"))?;

    let val: Value = resp
        .body_mut()
        .read_json()
        .context("Invalid JSON from LierSync Bridge")?;

    if val.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let msg = val
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown LierSync error");
        anyhow::bail!("{msg}");
    }

    Ok(LierSyncSendResult {
        success: true,
        target: val
            .get("target")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        sent_files: val.get("sent_files").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
        bytes: val.get("bytes").and_then(|v| v.as_u64()).unwrap_or(0),
    })
}

pub fn push_clipboard_to_liersync(text: Option<&str>, image_base64: Option<&str>) -> Result<()> {
    let url = format!("http://127.0.0.1:{LIERSYNC_BRIDGE_PORT}/api/ecosystem/clipboard");
    let body = json!({
        "text": text,
        "image_base64": image_base64,
        "source": "Lierkour"
    });
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .into();

    let mut resp = agent
        .post(&url)
        .send_json(&body)
        .map_err(|e| anyhow::anyhow!("LierSync недоступен на :{LIERSYNC_BRIDGE_PORT} — {e}"))?;

    let val: Value = resp
        .body_mut()
        .read_json()
        .context("Invalid JSON from LierSync Bridge")?;

    if val.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let msg = val
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown LierSync error");
        anyhow::bail!("{msg}");
    }
    Ok(())
}

pub fn forward_ost_to_liertags(
    title: &str,
    original_title: &str,
    creator: &str,
    year: Option<u16>,
    genres: &str,
    film_id: i64,
) -> Result<()> {
    let url = format!("http://127.0.0.1:{LIERTAGS_PORT}/api/ost/from-lierkour");
    let body = json!({
        "title": title,
        "original_title": if original_title.trim().is_empty() { None } else { Some(original_title) },
        "creator": if creator.trim().is_empty() { None } else { Some(creator) },
        "year": year.map(u32::from),
        "genres": if genres.trim().is_empty() { None } else { Some(genres) },
        "source_url": format!("lier://kour/{film_id}")
    });
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .into();

    agent
        .post(&url)
        .send_json(&body)
        .map_err(|e| anyhow::anyhow!("LierTags недоступен на :{LIERTAGS_PORT} — {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_probe_lists_four_apps() {
        let apps = probe_ecosystem();
        assert_eq!(apps.len(), 4);
        assert_eq!(apps[1].port, LIERKOUR_SYNC_PORT);
    }
}
