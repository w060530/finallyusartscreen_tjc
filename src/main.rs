#![no_std]
#![no_main]


mod protocol;
mod screen;
mod command;

use embassy_executor::Spawner;
use embassy_stm32::bind_interrupts;
use embassy_stm32::dma;
use embassy_stm32::peripherals;
use embassy_stm32::usart::{self, Config as UartConfig, Uart, UartTx};
use embassy_time::{Duration, Timer};
use embedded_hal_nb::nb;
use embedded_hal_nb::serial::Read;
use command::RingBuffer;
use protocol::{handle_fine, handle_material, handle_rough, handle_scan, Stage};
use screen::{Object, Screen, UsartHandle};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USART1       => usart::InterruptHandler<peripherals::USART1>;
    DMA2_STREAM2 => dma::InterruptHandler<peripherals::DMA2_CH2>;
    DMA2_STREAM7 => dma::InterruptHandler<peripherals::DMA2_CH7>;
    DMA1_STREAM6 => dma::InterruptHandler<peripherals::DMA1_CH6>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    defmt::info!("usartscreen boot");

    let p = embassy_stm32::init(Default::default());

    // 电脑通信：USART1 PA9=TX, PA10=RX（115200）
    let uart = Uart::new(p.USART1, p.PA10, p.PA9, p.DMA2_CH7, p.DMA2_CH2, Irqs, UartConfig::default()).unwrap();
    let (mut tx_pc,  rx) = uart.split();
    let rx_buf: &'static mut [u8] = unsafe {
        static mut BUF: [u8; 256] = [0u8; 256];
        &mut *(&raw mut BUF)
    };
    let mut rx = rx.into_ring_buffered(rx_buf);
    rx.start_uart();
    // 串口屏：USART2 PD5 发送指令（9600）
    let mut screen_cfg = UartConfig::default();
    screen_cfg.baudrate = 9600;
    let tx = UartTx::new(p.USART2, p.PD5, p.DMA1_CH6, Irqs, screen_cfg).unwrap();

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

    // 循环缓冲（接收字节）+ 完整帧缓冲
    let mut ring = RingBuffer::new();
    let mut cmd_buf = [0u8; 64];

    // 当前协议阶段 + 各阶段暂存
    let mut stage = Stage::Scan;
    let mut x: i16 = 0;
    let mut y: i16 = 0;
    let mut pts: [(i16, i16); 3] = [(0, 0); 3];
    let mut fx: i16 = 0;
    let mut fy: i16 = 0;
    let mut scan_result: Option<protocol::ScanResult> = None;

    // 
        // 非阻塞收帧 → 按阶段处理
    loop {
        // 1) 非阻塞把 DMA 缓冲里当前已有的字节搬进 RingBuffer
        loop {
            match Read::read(&mut rx) {
                Ok(b) => { ring.write(&[b]); }
                Err(nb::Error::WouldBlock) => break,
                Err(nb::Error::Other(_)) => { rx.start_uart(); break; }
            }
        }
        // 2) 从 RingBuffer 取完整帧，逐帧处理
        loop {
            let n = ring.get_command(&mut cmd_buf);
            if n == 0 { break; }
            let body = &cmd_buf[1..n - 1];
            stage = match stage {
                Stage::Scan => {
                    let (new_stage, result) = handle_scan(body, &mut screen, &mut tx_pc).await;
                    scan_result = result;
                    new_stage
                }
                Stage::Material => handle_material(body, &mut x, &mut y, &mut tx_pc).await,
                Stage::Rough => handle_rough(body, &mut pts, &mut tx_pc).await,
                Stage::Fine => handle_fine(body, &mut fx, &mut fy, &mut tx_pc).await,
            };
        }
        // 3) 让出 CPU，避免忙等
        Timer::after(Duration::from_millis(1)).await;
    }

}
