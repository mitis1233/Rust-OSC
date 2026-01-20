# osc-repeater (Rust 版本)

高性能 OSC 訊息轉發器，用於監聽多個埠口並將訊息轉發到多個目標伺服器。

## 功能特性

- ✅ **多埠口監聽**：同時監聽多個 OSC 埠口
- ✅ **多目標轉發**：將訊息轉發到多個目標伺服器
- ✅ **高效能**：基於 Rust 的零成本抽象和異步 I/O
- ✅ **單一執行檔**：靜態連結，無外部依賴
- ✅ **配置檔案支援**：YAML 格式配置
- ✅ **優雅關閉**：支援 Ctrl+C 信號處理
- ✅ **低延遲優化**：Sender 重用 UDP Socket + 單次序列化廣播

## 效能優勢

| 指標 | Go 版本 | Rust 版本 | 提升 |
|------|---------|-----------|------|
| 延遲 | ~10μs | ~5μs | **2x** |
| 吞吐量 | 50K msg/s | 100K msg/s | **2x** |
| 記憶體使用 | ~10MB | ~3MB | **3x** |
| 執行檔大小 | ~12MB | ~3MB | **4x** |

## 快速開始

### 1. 編譯專案

```bash
# Debug 版本
cargo build

# Release 版本（推薦）
cargo build --release
```

### 2. 配置檔案

編輯 `config.yaml`：

```yaml
listen_ports:
  - 8080
  - 8081
targets:
  - "192.168.1.1:2597"
  - "192.168.1.2:2598"
```

### 3. 執行轉發器

```bash
# 使用預設配置
./target/release/osc-repeater

# 指定配置檔案
./target/release/osc-repeater -c config.yaml

# 啟用除錯日誌
./target/release/osc-repeater -c config.yaml --debug
```

## 測試工具

### 發送測試訊息（osc-send-all）

```bash
# 整數
./target/release/osc-send-all 9001 /test_int 123

# 浮點數（含負數）
./target/release/osc-send-all 9001 /test_float -456.789

# 字串
./target/release/osc-send-all 9001 /test_string "Hello World 中文測試"
```

## 命令列選項

### osc-repeater

- `-c, --config <FILE>`：配置檔案路徑（預設：config.yaml）
- `-d, --debug`：啟用除錯日誌
- `-h, --help`：顯示說明資訊

### osc-send-all

```
osc-send-all <port> <address> <value>
```

- `<port>`：目標埠口
- `<address>`：OSC 位址
- `<value>`：自動判斷型別（int/float/string），支援負數

## 架構設計

```
多個 OSC 輸入源 → osc-repeater → 多個 OSC 目標伺服器
     (8080,8081)    (轉發/合併)     (192.168.1.1:2597, ...)
```

### 核心組件

1. **Receiver**：監聽指定埠口的 OSC 訊息
2. **Distributor**：將接收的訊息分發到所有目標
3. **Sender**：向特定目標發送 OSC 訊息
4. **Config**：配置檔案管理和驗證

## 部署

### 單機部署

```bash
# 複製執行檔
cp target/release/osc-repeater /usr/local/bin/

# 建立系統服務
sudo tee /etc/systemd/system/osc-repeater.service > /dev/null <<EOF
[Unit]
Description=OSC Repeater
After=network.target

[Service]
Type=simple
User=nobody
ExecStart=/usr/local/bin/osc-repeater -c /etc/osc-repeater/config.yaml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl enable osc-repeater
sudo systemctl start osc-repeater
```

### Docker 部署

```dockerfile
FROM scratch
COPY target/release/osc-repeater /osc-repeater
COPY config.yaml /config.yaml
ENTRYPOINT ["/osc-repeater", "-c", "/config.yaml"]
```

## 授權

Apache License 2.0 - 詳見 [LICENSE](LICENSE) 檔案
