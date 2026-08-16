#![no_std]
#![no_main]

mod input;
mod matcher;
mod screen;

use embassy_executor::Spawner;
use embassy_stm32::mode::Blocking;
use embassy_stm32::usart::{Config as UartConfig, UartRx, UartTx};
use embassy_time::{Duration, Timer};
use input::InputBuf;
use matcher::Tostr;
use screen::{Object, Screen, SerialHandle};
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

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    defmt::info!("usartscreen boot");

    // 时钟用默认值（HSI 16MHz），与 ec 项目一致
    let p = embassy_stm32::init(Default::default());

    // USART1：PB7 接收扫码枪（115200，默认值）
    let mut rx = UartRx::new_blocking(p.USART1, p.PB7, UartConfig::default()).unwrap();
    // USART2：PD5 发送指令到串口屏（9600，匹配屏幕出厂默认波特率）
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


    // 运行时改写 context：三个控件都改成 123456
    screen.objects[0].set_context(b"123456");
    screen.objects[1].set_context(b"123456");
    screen.objects[2].set_context(b"123456");
    // 把改好的 context 读出来，拼指令发到屏
    Timer::after(Duration::from_millis(1000)).await;
    screen.serial.refresh(&screen.objects).unwrap();

    let mut input = InputBuf::new();
    let mut byte = [0u8; 1];
    let mut idx = 0usize;
    
    // 收三行 → 自动识别类型 → 依次填 t0/t1/t2 → 收满 refresh 发屏
    loop {

         screen.serial.refresh(&screen.objects).unwrap();
         Timer::after(Duration::from_secs(1000)).await;
        // rx.blocking_read(&mut byte).unwrap();

        // if let Some(line) = input.feed(byte[0]) {
        //     defmt::info!("recv: {}", line);

        //     fill_auto(line, &mut screen.objects[idx]);
        //     idx += 1;
        //     if idx >= 3 {
        //         screen.serial.refresh(&screen.objects).unwrap();
        //         idx = 0;
        //     }
        // }
    }
}
