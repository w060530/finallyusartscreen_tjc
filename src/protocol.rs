//! 协议状态机：扫码 → 接收物料 → 粗定位 → 细精度 四个阶段。


use embassy_stm32::mode::Async;
use embassy_stm32::usart::UartTx;
use embassy_time::{Duration, Timer};

use crate::screen::{RefreshOne, Screen, UsartHandle};
/// 把两个字节拼成一个 i16（高字节在前、低字节在后）。
fn u8_to_i16(high: u8, low: u8) -> i16 {
    let combined = ((high as u16) << 8) | (low as u16);
    combined as i16
}

pub struct Task{
    pub pick_color: [u8;3],
    pub pick_place: [u8;3],
}
pub struct ScanResult{
    pub firststep: Task,
    pub secondstep: Task,
    
}
impl Task {
    pub fn parse(data: &[u8]) -> Option<Task> { 
        for i in 0..3 {
            if data[i] < b'0' || data[i] > b'5' {
                return None;
            }}
        for i in 3..6 {
                if data[i] < b'1' || data[i] > b'3' {
                    return None;}
                }
        Some(Task{pick_color: [data[0] - b'0', data[1] - b'0', data[2] - b'0'],
            pick_place: [data[3] - b'0', data[4] - b'0', data[5] - b'0'],

                })
            
            
        
        
     }
}

impl ScanResult {
    pub fn parse(body: &[u8]) -> Option<ScanResult> {
        if body.len() != 13 || body[6] != b'+' {
            return None;
        }
        let firststep = Task::parse(&body[0..6])?;
        let secondstep = Task::parse(&body[7..13])?;
        Some(ScanResult { firststep, secondstep })
    }
}

/// 协议阶段（运行时状态机：enum + match，非纯类型状态机）。
#[derive(Clone, Copy, PartialEq)]
pub enum Stage {
    Scan,     // 阶段1：扫码（等二维码帧 "012345+543210"）
    Material, // 阶段2：接收物料（color(0~5) + X(i16) + Y(i16)）
    Rough,    // 阶段3：粗定位（3 个坐标点）
    Fine,     // 阶段4：细精度（1 个坐标点）
}

/// 屏类型别名：3 个控件 + 真串口句柄 UsartHandle。
pub type AppScreen<'d> = Screen<'static, 3, 64, UsartHandle<'d>>;

/// 阶段1：扫码。收到二维码帧后屏打印、回发 S、延时 1s、回发 M，进入接收物料阶段。
pub async fn handle_scan<'d>(
    body: &[u8],
    screen: &mut AppScreen<'d>,
    tx_pc: &mut UartTx<'_, Async>,
) ->( Stage ,Option<ScanResult>) {
match ScanResult::parse(body) {
        Some(scan_result) => {
            defmt::info!("scan result: firststep color={:?} place={:?}, secondstep color={:?} place={:?}",
                scan_result.firststep.pick_color, scan_result.firststep.pick_place,
                scan_result.secondstep.pick_color, scan_result.secondstep.pick_place);

            
            let content = core::str::from_utf8(body).unwrap_or("invalid");
            screen.objects[0].set_context(content.as_bytes());
            screen.serial.refresh_one(&screen.objects[0]).unwrap();

            
            tx_pc.write(b"S").await.unwrap();
            Timer::after(Duration::from_millis(1000)).await;
            tx_pc.write(b"M").await.unwrap();

            (Stage::Material, Some(scan_result))
        }
        None => {
            defmt::warn!("invalid scan frame: {:?}", body);
            (Stage::Scan, None)
        }
    }
    
}


/// 阶段2：接收物料。body = [color, x_hi, x_lo, y_hi, y_lo]，拼 i16 存 x/y，回发 N，进入粗定位阶段。
pub async fn handle_material(
    body: &[u8],
    x: &mut i16,
    y: &mut i16,
    tx_pc: &mut UartTx<'_, Async>,
) -> Stage {
    if body.len() == 5 && body[0] <= 5 {
        *x = u8_to_i16(body[1], body[2]);
        *y = u8_to_i16(body[3], body[4]);
        defmt::info!("material color={} x={} y={}", body[0], *x, *y);

        // 回发 'N'（Python 收到后进入阶段3）
        tx_pc.write(b"N").await.unwrap();

        Stage::Rough
    } else {
        Stage::Material
    }
}

/// 阶段3：粗定位。body = 3 个点，每个点 [X高,X低,Y高,Y低]，共 12 字节，存 pts 数组，回发 C，进入细精度阶段。
pub async fn handle_rough(
    body: &[u8],
    pts: &mut [(i16, i16); 3],
    tx_pc: &mut UartTx<'_, Async>,
) -> Stage {
    if body.len() == 12 {
        for i in 0..3 {
            let off = i * 4;
            pts[i] = (
                u8_to_i16(body[off], body[off + 1]),
                u8_to_i16(body[off + 2], body[off + 3]),
            );
        }
        defmt::info!(
            "points p0=({},{}) p1=({},{}) p2=({},{})",
            pts[0].0, pts[0].1, pts[1].0, pts[1].1, pts[2].0, pts[2].1
        );

        // 回发 'C'（Python 收到后进入阶段4）
        tx_pc.write(b"C").await.unwrap();

        Stage::Fine
    } else {
        Stage::Rough
    }
}

/// 阶段4：细精度。body = [X高,X低,Y高,Y低]，1 个点，存 fx/fy，回发 X，协议结束（停在 Fine 不再切换）。
pub async fn handle_fine(
    body: &[u8],
    fx: &mut i16,
    fy: &mut i16,
    tx_pc: &mut UartTx<'_, Async>,
) -> Stage {
    if body.len() == 4 {
        *fx = u8_to_i16(body[0], body[1]);
        *fy = u8_to_i16(body[2], body[3]);
        defmt::info!("fine x={} y={}", *fx, *fy);

        // 回发 'X'（Python 收到后结束整个流程）
        tx_pc.write(b"X").await.unwrap();
    }
    // 协议到此结束，停在 Fine 不再切换
    Stage::Fine
}
