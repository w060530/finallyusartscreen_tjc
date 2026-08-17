//! 帧解析：以 0xAA 为帧头、0xBB 为帧尾，从串口字节流里识别完整帧。

/// 最大帧总长：阶段1 body 13 字节 + 头尾 2 = 15（所有阶段里最长）
pub const MAX_FRAME_LEN: usize = 15;
/// 接收缓冲大小 = 最大帧长 × 2 − 1 = 29：最坏残留半帧 + 新到完整一帧，绝不溢出
pub const BUF_LEN: usize = MAX_FRAME_LEN * 2 - 1;

// ===== 旧常量保留（当时只按阶段1 二维码帧长 14 算，现已改为按最大帧长 15 算） =====
// pub const FRAME_LEN: usize = 14;
// pub const BUF_LEN: usize = FRAME_LEN * 2 - 1;
// ===== 旧常量结束 =====

/// 把两个字节拼成一个 i16（高字节在前、低字节在后）。
/// 坐标传输时把 i16 拆成高 8 位 / 低 8 位两个字节，这里拼回来；负数也能正确还原。
pub fn u8_to_i16(high: u8, low: u8) -> i16 {
    let combined = ((high as u16) << 8) | (low as u16);
    combined as i16
}

/// 逐字节帧解析器。
pub struct FrameParser {
    buf: [u8; BUF_LEN],
    len: usize,
    in_frame: bool,
}

impl FrameParser {
    pub const fn new() -> Self {
        Self {
            buf: [0u8; BUF_LEN],
            len: 0,
            in_frame: false,
        }
    }

    /// 喂入一个字节。识别到完整一帧（0xAA ... 0xBB）时返回 true，
    /// 之后可调 `body()` 读取该帧 body（不含帧头帧尾）。
    pub fn feed(&mut self, byte: u8) -> bool {
        if byte == 0xAA {
            // 帧头：开始新帧，残缺的上一帧被覆盖丢弃
            self.buf[0] = 0xAA;
            self.len = 1;
            self.in_frame = true;
            return false;
        }

        if !self.in_frame {
            // 还没遇到帧头，丢弃
            return false;
        }

        if byte == 0xBB {
            // 帧尾：一帧完整
            self.in_frame = false;
            if self.len < BUF_LEN {
                self.buf[self.len] = 0xBB;
                self.len += 1;
            }
            return true;
        }

        // 普通 body 字节，存入缓冲（预留帧尾位置）
        if self.len < BUF_LEN - 1 {
            self.buf[self.len] = byte;
            self.len += 1;
        }
        false
    }

    /// 最近一帧的 body（不含 0xAA 帧头、0xBB 帧尾）。
    pub fn body(&self) -> &[u8] {
        if self.len >= 2 {
            &self.buf[1..self.len - 1]
        } else {
            &[]
        }
    }
}
