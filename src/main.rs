use clap::Parser;
use osc_repeater::{Config, OscRepeater};
use std::sync::Arc;
use tokio::signal;
use tracing::{info, error};

#[cfg(windows)]
use windows_sys::Win32::System::Console::GetConsoleWindow;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_MINIMIZE};

#[derive(Parser)]
#[command(name = "osc-repeater")]
#[command(about = "高效能 OSC 訊號轉發器")]
struct Args {
    /// 配置檔案路徑
    #[arg(short, long, default_value = "config.yaml")]
    config: String,

    /// 啟用除錯日誌
    #[arg(short, long)]
    debug: bool,
}

#[cfg(windows)]
fn minimize_console_window() {
    unsafe {
        let hwnd = GetConsoleWindow();
        if hwnd != std::ptr::null_mut() {
            ShowWindow(hwnd, SW_MINIMIZE);
        }
    }
}

#[cfg(not(windows))]
fn minimize_console_window() {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    minimize_console_window();

    // 初始化日誌系統
    let log_level = if args.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("osc_repeater={}", log_level))
        .with_ansi(false)  // 禁用 ANSI 顏色代碼
        .init();

    // 載入配置檔案
    let config = match Config::load_from_file(&args.config) {
        Ok(config) => {
            info!("配置檔案: {}", args.config);
            info!("監聽埠口: {:?}", config.listen_ports);
            info!("目標: {:?}", config.targets);
            config
        }
        Err(e) => {
            error!("配置錯誤: {}", e);
            return Err(e.into());
        }
    };

    // 建立 OSC 轉發器
    let repeater = Arc::new(OscRepeater::new(config));

    // 設置信號處理
    let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // 處理 Ctrl+C 信號
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Ctrl+C 處理器失敗");
    };

    tokio::select! {
        result = repeater.run() => {
            if let Err(e) = result {
                error!("轉發器錯誤: {}", e);
                return Err(e);
            }
        }
        _ = ctrl_c => {
            info!("收到 Ctrl+C，關閉中...");
        }
        _ = shutdown_rx => {
            info!("收到關閉信號");
        }
    }

    info!("OSC 轉發器已停止");
    Ok(())
}
