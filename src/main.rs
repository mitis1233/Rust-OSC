use clap::Parser;
use osc_repeater::{Config, OscRepeater, OscMessage, OscArg};
use std::sync::Arc;
use tokio::signal;
use tracing::{info, error};

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

    /// 測試 OSC 序列化
    #[arg(long)]
    test: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // 測試 OSC 序列化（如果請求）
    if args.test {
        test_osc_serialization();
        return Ok(());
    }

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

fn test_osc_serialization() {
    println!("測試 OSC 序列化...");
    
    // 測試基本訊息
    let message = OscMessage::new("/test").push_int(101);
    let serialized = message.serialize();
    
    println!("序列化結果: {:?}", serialized);
    println!("十六進位: {:02x?}", serialized);
    
    // 測試反序列化
    match OscMessage::deserialize(&serialized) {
        Ok(deserialized) => {
            println!("✓ 位址: {}", deserialized.address);
            println!("✓ 參數: {:?}", deserialized.args);
        }
        Err(e) => {
            println!("✗ 失敗: {}", e);
        }
    }
    
    // 測試負數
    println!("\n測試負數處理...");
    
    // 負整數
    let neg_int_msg = OscMessage::new("/neg_int").push_int(-123);
    let neg_int_serialized = neg_int_msg.serialize();
    match OscMessage::deserialize(&neg_int_serialized) {
        Ok(deserialized) => {
            println!("✓ 負整數: {} -> {:?}", deserialized.address, deserialized.args);
        }
        Err(e) => {
            println!("✗ 負整數失敗: {}", e);
        }
    }
    
    // 負浮點數
    let neg_float_msg = OscMessage::new("/neg_float").push_float(-456.789);
    let neg_float_serialized = neg_float_msg.serialize();
    match OscMessage::deserialize(&neg_float_serialized) {
        Ok(deserialized) => {
            println!("✓ 負浮點數: {} -> {:?}", deserialized.address, deserialized.args);
        }
        Err(e) => {
            println!("✗ 負浮點數失敗: {}", e);
        }
    }
    
    // 測試多參數訊息
    let message2 = OscMessage::new("/multi")
        .push_int(42)
        .push_float(3.14)
        .push_string("hello");
    let serialized2 = message2.serialize();
    
    println!("\n測試多參數...");
    match OscMessage::deserialize(&serialized2) {
        Ok(deserialized) => {
            println!("✓ 位址: {}", deserialized.address);
            for (i, arg) in deserialized.args.iter().enumerate() {
                match arg {
                    OscArg::Int(v) => println!("  參數 {}: 整數 {}", i, v),
                    OscArg::Float(v) => println!("  參數 {}: 浮點數 {}", i, v),
                    OscArg::String(v) => println!("  參數 {}: 字串 {}", i, v),
                    OscArg::Bool(v) => println!("  參數 {}: 布林值 {}", i, v),
                }
            }
        }
        Err(e) => {
            println!("✗ 失敗: {}", e);
        }
    }
}
