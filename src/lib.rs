use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use thiserror::Error;
use serde::{Deserialize, Serialize};

const MAX_PACKET_SIZE: usize = 65507;
type OscPayload = Arc<[u8]>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub listen_ports: Vec<u16>,
    pub targets: Vec<SocketAddr>,
}

impl Config {
    pub fn load_from_file(path: &str) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.listen_ports.is_empty() {
            return Err(ConfigError::NoListenPorts);
        }
        
        if self.targets.is_empty() {
            return Err(ConfigError::NoTargets);
        }
        
        for port in &self.listen_ports {
            if *port == 0 {
                return Err(ConfigError::InvalidPort(*port));
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO 錯誤: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML 解析錯誤: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("未指定監聽埠口")]
    NoListenPorts,
    #[error("未指定目標地址")]
    NoTargets,
    #[error("無效的埠口: {0}")]
    InvalidPort(u16),
}

#[derive(Debug, Clone)]
pub struct OscMessage {
    pub address: String,
    pub args: Vec<OscArg>,
}

#[derive(Debug, Clone)]
pub enum OscArg {
    Int(i32),
    Float(f32),
    String(String),
    Bool(bool),
}

impl OscMessage {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            args: Vec::new(),
        }
    }

    pub fn push_int(mut self, value: i32) -> Self {
        self.args.push(OscArg::Int(value));
        self
    }

    pub fn push_float(mut self, value: f32) -> Self {
        self.args.push(OscArg::Float(value));
        self
    }

    pub fn push_string(mut self, value: impl Into<String>) -> Self {
        self.args.push(OscArg::String(value.into()));
        self
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        
        // OSC address pattern (null-terminated)
        buffer.extend_from_slice(self.address.as_bytes());
        buffer.push(0);
        
        // Align to 4-byte boundary
        while buffer.len() % 4 != 0 {
            buffer.push(0);
        }
        
        // Type tags
        buffer.push(b',');
        for arg in &self.args {
            match arg {
                OscArg::Int(_) => buffer.push(b'i'),
                OscArg::Float(_) => buffer.push(b'f'),
                OscArg::String(_) => buffer.push(b's'),
                OscArg::Bool(value) => buffer.push(if *value { b'T' } else { b'F' }),
            }
        }
        buffer.push(0);
        
        // Align to 4-byte boundary
        while buffer.len() % 4 != 0 {
            buffer.push(0);
        }
        
        // Arguments
        for arg in &self.args {
            match arg {
                OscArg::Int(value) => {
                    buffer.extend_from_slice(&value.to_be_bytes());
                }
                OscArg::Float(value) => {
                    buffer.extend_from_slice(&value.to_be_bytes());
                }
                OscArg::String(value) => {
                    buffer.extend_from_slice(value.as_bytes());
                    buffer.push(0);
                    while buffer.len() % 4 != 0 {
                        buffer.push(0);
                    }
                }
                OscArg::Bool(_) => {
                    // 'T'/'F' tags do not carry argument data
                }
            }
        }
        
        buffer
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 4 {
            return Err("資料長度不足以解析 OSC 訊息");
        }

        // Find address (null-terminated string)
        let mut addr_end = 0;
        while addr_end < data.len() && data[addr_end] != 0 {
            addr_end += 1;
        }
        
        if addr_end >= data.len() || addr_end == 0 {
            return Err("無效的位址格式");
        }

        let address = String::from_utf8_lossy(&data[..addr_end]).to_string();
        
        // Skip to type tags (aligned to 4-byte boundary)
        let mut pos = ((addr_end + 1 + 3) / 4) * 4;
        if pos >= data.len() || data[pos] != b',' {
            return Err("無效的類型標籤格式");
        }
        
        // Parse type tags
        let mut type_end = pos + 1;
        while type_end < data.len() && data[type_end] != 0 {
            type_end += 1;
        }
        
        if type_end >= data.len() {
            return Err("類型標籤未終止");
        }
        
        let type_tags = &data[pos + 1..type_end];
        
        // Skip to arguments (aligned to 4-byte boundary)
        pos = ((type_end + 1 + 3) / 4) * 4;
        
        let mut args = Vec::new();
        for &type_tag in type_tags {
            match type_tag {
                b'i' => {
                    if pos + 4 > data.len() {
                        return Err("整數參數資料不足");
                    }
                    let bytes = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
                    let value = i32::from_be_bytes(bytes);
                    args.push(OscArg::Int(value));
                    pos += 4;
                }
                b'f' => {
                    if pos + 4 > data.len() {
                        return Err("浮點數參數資料不足");
                    }
                    let bytes = [data[pos], data[pos + 1], data[pos + 2], data[pos + 3]];
                    let value = f32::from_be_bytes(bytes);
                    args.push(OscArg::Float(value));
                    pos += 4;
                }
                b's' => {
                    if pos >= data.len() {
                        return Err("字串參數資料不足");
                    }
                    let mut str_end = pos;
                    while str_end < data.len() && data[str_end] != 0 {
                        str_end += 1;
                    }
                    if str_end >= data.len() {
                        return Err("字串參數未終止");
                    }
                    let value = String::from_utf8_lossy(&data[pos..str_end]).to_string();
                    args.push(OscArg::String(value));
                    pos = ((str_end + 1 + 3) / 4) * 4;
                }
                b'T' | b'F' => {
                    // Boolean values don't have data in the argument section
                    args.push(OscArg::Bool(type_tag == b'T'));
                }
                _ => return Err("未知的類型標籤"),
            }
        }
        
        Ok(OscMessage { address, args })
    }
}

pub struct OscRepeater {
    config: Config,
    distributor: Arc<Distributor>,
}

impl OscRepeater {
    pub fn new(config: Config) -> Self {
        let distributor = Arc::new(Distributor::new(&config.targets));
        Self { config, distributor }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut handles = Vec::new();

        // Start sender tasks
        for sender in &self.distributor.senders {
            let rx = self.distributor.subscribe();
            let sender = sender.clone();
            let handle = tokio::spawn(async move {
                sender.run(rx).await;
            });
            handles.push(handle);
        }

        // Create receivers for each listen port
        for &port in &self.config.listen_ports {
            let distributor = self.distributor.clone();
            let handle = tokio::spawn(async move {
                if let Err(e) = Receiver::new(port, distributor).run().await {
                    eprintln!("埠口 {} 錯誤: {}", port, e);
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await?;
        }

        Ok(())
    }
}

pub struct Distributor {
    senders: Vec<Sender>,
    tx: broadcast::Sender<OscPayload>,
}

impl Distributor {
    pub fn new(targets: &[SocketAddr]) -> Self {
        let (tx, _) = broadcast::channel(1000);
        let mut senders = Vec::new();

        for &target in targets {
            senders.push(Sender::new(target));
        }

        Self { senders, tx }
    }

    pub fn send(&self, payload: OscPayload) {
        let _ = self.tx.send(payload);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<OscPayload> {
        self.tx.subscribe()
    }
}

#[derive(Debug, Clone)]
pub struct Sender {
    target: SocketAddr,
}

impl Sender {
    pub fn new(target: SocketAddr) -> Self {
        Self { target }
    }

    pub async fn run(self, mut rx: broadcast::Receiver<OscPayload>) {
        let socket = match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
            Ok(socket) => socket,
            Err(e) => {
                eprintln!("建立送出 Socket 失敗: {}", e);
                return;
            }
        };

        if let Err(e) = socket.connect(self.target).await {
            eprintln!("連線到 {} 失敗: {}", self.target, e);
            return;
        }

        loop {
            match rx.recv().await {
                Ok(payload) => {
                    if let Err(e) = socket.send(payload.as_ref()).await {
                        eprintln!("發送失敗: {}", e);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    }
}

pub struct Receiver {
    port: u16,
    distributor: Arc<Distributor>,
}

impl Receiver {
    pub fn new(port: u16, distributor: Arc<Distributor>) -> Self {
        Self { port, distributor }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("0.0.0.0:{}", self.port);
        let socket = tokio::net::UdpSocket::bind(&addr).await?;
        
        println!("監聽埠口: {}", addr);

        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        loop {
            let len = match socket.recv_from(&mut buf).await {
                Ok((len, _src)) => len,
                Err(e) => {
                    eprintln!("接收錯誤: {}", e);
                    continue;
                }
            };

            if let Err(e) = OscMessage::deserialize(&buf[..len]) {
                eprintln!("解析錯誤: {}", e);
                continue;
            }

            let payload: OscPayload = Arc::from(&buf[..len]);
            self.distributor.send(payload);
        }
    }
}
