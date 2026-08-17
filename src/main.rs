#![no_std]
#![no_main]

mod frame;
mod screen;
// mod input;   // 旧：按换行 \n 拆分的输入缓冲，已改用 frame.rs，暂注释
// mod matcher; // 旧：Tostr 转字符串 + 题目匹配，暂未使用（下方 fill_auto 依赖它）

use embassy_executor::Spawner;
use embassy_stm32::mode::Blocking;
use embassy_stm32::usart::{Config as UartConfig, Uart, UartTx};
use embassy_time::{Duration, Timer};
use frame::{u8_to_i16, FrameParser};
use screen::{Object, RefreshOne, Screen, SerialHandle};
use {defmt_rtt as _, panic_probe as _};

/// 真串口实现：把 UartTx 包装成 SerialHandle，refresh 时把每个控件的指令发到串口屏。
struct UsartHandle<'d> {
    tx: UartTx<'d, Blocking>,
}

impl<'d, const N: usize, const T: usize> SerialHandle<N, T> for UsartHandle<'d> {
    fn refresh(&mut self, objs: &[Object<'_, T>; N]) -> Result<(), ()> {
        let mut buf = [0u8; 256];
        for obj in objs.iter() {
            let len = screen::build_cmd(obj.name, &obj.context, obj.len, &mut buf);
            self.tx.blocking_write(&buf[..len]).map_err(|_| ())?;
        }
        Ok(())
    }
}

/// 单控件刷新：只发一个控件，不动其余控件。
impl<'d, const T: usize> RefreshOne<T> for UsartHandle<'d> {
    fn refresh_one(&mut self, obj: &Object<'_, T>) -> Result<(), ()> {
        let mut buf = [0u8; 256];
        let len = screen::build_cmd(obj.name, &obj.context, obj.len, &mut buf);
        self.tx.blocking_write(&buf[..len]).map_err(|_| ())
    }
}

/*
 * ===== 旧代码保留：自动识别类型转字符串（以后解析串口输入时可能用到） =====
 * 注意：若要启用，需同时恢复 mod matcher; 和 use matcher::Tostr;
 *
/// 自动识别类型：整数 → 浮点 → 布尔 → 文本，转成字符串填入控件。
fn fill_auto<const T: usize>(line: &str, obj: &mut Object<'_, T>) {
    let mut buf = [0u8; 64];

    // ① 整数
    if let Ok(v) = line.parse::<i32>() {
        let s = v.to_str(&mut buf);
        obj.set_context(s.as_bytes());
        return;
    }

    // ② 浮点
    if let Ok(v) = line.parse::<f64>() {
        let s = v.to_str(&mut buf);
        obj.set_context(s.as_bytes());
        return;
    }

    // ③ 布尔
    match line {
        "true" | "1" => {
            obj.set_context(b"true");
            return;
        }
        "false" | "0" => {
            obj.set_context(b"false");
            return;
        }
        _ => {}
    }

    // ④ 文本
    obj.set_context(line.as_bytes());
}
 * ===== fill_auto 结束 =====
 */

/// 协议阶段（运行时状态机：enum + match，非纯类型状态机）。
#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Qr,     // 阶段1：等二维码帧 "012345+543210"
    Rough,  // 阶段2：等粗糙位置帧 color(0~5) + X(i16) + Y(i16)
    Points, // 阶段3：等 3 个坐标点帧（暂未实现）
    Fine,   // 阶段4：等细定位帧（暂未实现）
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    defmt::info!("usartscreen boot");

    // 时钟用默认值（HSI 16MHz），与 ec 项目一致
    let p = embassy_stm32::init(Default::default());

    // 电脑通信：USART1 的 PA9=TX 回发 S/M，PA10=RX 收 Python 帧（115200 默认）
    // 同一串口同时收发必须用 Uart::new_blocking 一次性创建再 split，否则 p.USART1 会被 move 两次
    // 注意 new_blocking 参数顺序是 (peri, rx, tx, config)
    let uart = Uart::new_blocking(p.USART1, p.PA10, p.PA9, UartConfig::default()).unwrap();
    let (mut tx_pc, mut rx) = uart.split();

    // 原扫码枪接 USART1 PB7，现改为电脑（PA9/PA10），此段注释保留：
    // let mut rx = UartRx::new_blocking(p.USART1, p.PB7, UartConfig::default()).unwrap();

    // 串口屏：USART2 PD5 发送指令（9600，匹配屏幕出厂默认波特率）
    let mut screen_cfg = UartConfig::default();
    screen_cfg.baudrate = 9600;
    let tx = UartTx::new_blocking(p.USART2, p.PD5, screen_cfg).unwrap();

    // 串口屏：3 个控件（t0/t1/t2）+ 真串口句柄
    let handle = UsartHandle { tx };
    let objects: [Object<'static, 64>; 3] = [
        Object::new("t0.txt", b""),
        Object::new("t1.txt", b""),
        Object::new("t2.txt", b""),
    ];
    let mut screen = Screen {
        serial: handle,
        objects,
    };

    // 帧解析器：以 0xAA 为帧头、0xBB 为帧尾
    let mut parser = FrameParser::new();
    let mut byte = [0u8; 1];

    // 当前协议阶段 + 阶段2 拼出来的坐标（存两个变量）
    let mut stage = Stage::Qr;
    let mut x: i16;
    let mut y: i16;

    // 收帧 → 按阶段处理
    loop {
        rx.blocking_read(&mut byte).unwrap();

        if parser.feed(byte[0]) {
            let body = parser.body();
            match stage {
                Stage::Qr => {
                    if body == &b"012345+543210"[..] {
                        defmt::info!("got QR frame");

                        // ① 串口屏打印 body（只发 t0，t1/t2 保持屏初始值 00000000 不动）
                        screen.objects[0].set_context(body);
                        screen.serial.refresh_one(&screen.objects[0]).unwrap();

                        // ② 回发 'S'（Python 收到后停止发 QR 帧）
                        tx_pc.blocking_write(b"S").unwrap();

                        // ③ 延时 1 秒
                        Timer::after(Duration::from_millis(1000)).await;

                        // ④ 回发 'M'（Python 收到后进入阶段2，开始发坐标帧）
                        tx_pc.blocking_write(b"M").unwrap();

                        stage = Stage::Rough;
                    }
                }
                Stage::Rough => {
                    // body = [color, x_hi, x_lo, y_hi, y_lo]，全数字值字节
                    if body.len() == 5 && body[0] <= 5 {
                        x = u8_to_i16(body[1], body[2]);
                        y = u8_to_i16(body[3], body[4]);
                        defmt::info!("rough color={} x={} y={}", body[0], x, y);

                        // 回发 'N'（Python 收到后进入阶段3）
                        tx_pc.blocking_write(b"N").unwrap();

                        stage = Stage::Points;
                    }
                }
                Stage::Points | Stage::Fine => {
                    // 阶段3/4 暂未实现
                }
            }
        }
    }
}
