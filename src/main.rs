#![no_std]
#![no_main]

mod input;

use embassy_executor::Spawner;
use embassy_stm32::usart::{Config as UartConfig, UartRx, UartTx};
use input::InputBuf;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    defmt::info!("usartscreen boot");

    // 时钟用默认值（HSI 16MHz），与 ec 项目一致
    let p = embassy_stm32::init(Default::default());

    // USART1：PB7 接收扫码枪（115200，默认值）
    let mut rx = UartRx::new_blocking(p.USART1, p.PB7, UartConfig::default()).unwrap();
    // USART2：PD5 发送指令到串口屏（115200，默认值）
    let mut tx = UartTx::new_blocking(p.USART2, p.PD5, UartConfig::default()).unwrap();

    let mut input = InputBuf::new();
    let mut byte = [0u8; 1];

    // 收发循环：收字节 → 攒行 → 匹配 → 拼指令 → 发送
    loop {
        rx.blocking_read(&mut byte).unwrap();

        if let Some(line) = input.feed(byte[0]) {
            if let Some(code) = question_code(line) {
                defmt::info!("match: {}", code);

                let mut cmd = [0u8; 32];
                let len = build_cmd("main.t0.txt", code, &mut cmd);
                tx.blocking_write(&cmd[..len]).unwrap();
            }
        }
    }
}

/// 「第1道题」→ Some("01")，匹配不上返回 None。
/// 注意：matcher.rs 里的 Question 目前是注释状态，这里先内联一份。
fn question_code(s: &str) -> Option<&'static str> {
    match s {
        "第1道题" => Some("01"),
        "第2道题" => Some("02"),
        "第3道题" => Some("03"),
        "第4道题" => Some("04"),
        "第5道题" => Some("05"),
        "第6道题" => Some("06"),
        _ => None,
    }
}

/// 拼 TJC 指令：name="code"\xFF\xFF\xFF，返回写入的字节数。
/// 注意：screen.rs 里的 build_cmd 是私有函数，这里先内联一份。
fn build_cmd(name: &str, code: &str, buf: &mut [u8]) -> usize {
    let mut i = 0;
    buf[i..i + name.len()].copy_from_slice(name.as_bytes());
    i += name.len();
    buf[i] = b'=';
    i += 1;
    buf[i] = b'"';
    i += 1;
    buf[i..i + code.len()].copy_from_slice(code.as_bytes());
    i += code.len();
    buf[i] = b'"';
    i += 1;
    buf[i] = 0xFF;
    i += 1;
    buf[i] = 0xFF;
    i += 1;
    buf[i] = 0xFF;
    i += 1;
    i
}
