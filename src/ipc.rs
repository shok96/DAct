use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::paths::data_dir;

const PORT_FILE: &str = "instance.port";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    #[serde(default)]
    pub files: Vec<PathBuf>,
    #[serde(default)]
    pub show: bool,
}

pub fn port_file() -> PathBuf {
    data_dir().join(PORT_FILE)
}

/// If another DAct is running, hand files over and return true.
pub fn handoff(msg: &IpcMessage) -> bool {
    let Ok(raw) = std::fs::read_to_string(port_file()) else {
        return false;
    };
    let Ok(port) = raw.trim().parse::<u16>() else {
        return false;
    };
    let Ok(mut stream) = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_millis(250),
    ) else {
        return false;
    };
    let _ = stream.set_write_timeout(Some(Duration::from_millis(400)));
    let Ok(body) = serde_json::to_vec(msg) else {
        return false;
    };
    stream.write_all(&body).is_ok() && stream.flush().is_ok()
}

pub fn listen(tx: Sender<IpcMessage>) -> anyhow::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    if let Some(parent) = port_file().parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(port_file(), port.to_string())?;
    listener.set_nonblocking(false)?;
    thread::Builder::new()
        .name("dact-ipc".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                if let Ok(msg) = read_message(stream) {
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
            }
        })?;
    Ok(port)
}

fn read_message(mut stream: TcpStream) -> anyhow::Result<IpcMessage> {
    stream.set_read_timeout(Some(Duration::from_millis(800)))?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    Ok(serde_json::from_slice(&buf)?)
}

pub fn cleanup() {
    let _ = std::fs::remove_file(port_file());
}
