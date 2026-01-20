use osc_repeater::OscMessage;
use std::net::SocketAddr;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 4 {
        println!("用法: osc-send-all <port> <address> <value>");
        println!("範例:");
        println!("  osc-send-all 9001 /test 123");
        println!("  osc-send-all 9001 /test -456.789");
        println!("  osc-send-all 9001 /test Hello");
        return Ok(());
    }
    
    let port: u16 = args[1].parse()?;
    let address = &args[2];
    let value = &args[3];
    
    let target: SocketAddr = format!("127.0.0.1:{}", port).parse()?;
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    
    let mut message = OscMessage::new(address);
    
    // 嘗試自動偵測類型
    if let Ok(int_val) = value.parse::<i32>() {
        message = message.push_int(int_val);
        println!("偵測為整數: {}", int_val);
    } else if let Ok(float_val) = value.parse::<f32>() {
        message = message.push_float(float_val);
        println!("偵測為浮點數: {}", float_val);
    } else {
        message = message.push_string(value.clone());
        println!("偵測為字串: {}", value);
    }
    
    let data = message.serialize();
    socket.send_to(&data, target).await?;
    
    println!("已發送到 {}: {}", target, address);
    for arg in &message.args {
        match arg {
            osc_repeater::OscArg::Int(v) => println!("  整數: {}", v),
            osc_repeater::OscArg::Float(v) => println!("  浮點數: {}", v),
            osc_repeater::OscArg::String(v) => println!("  字串: {}", v),
            osc_repeater::OscArg::Bool(v) => println!("  布林值: {}", v),
        }
    }
    
    Ok(())
}
