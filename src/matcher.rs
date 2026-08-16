
pub trait Tostr{
    fn to_str<'a>(&self,buf: &'a mut [u8])-> &'a str;

}

impl Tostr for &str {
    fn to_str<'a>(&self, buf: &'a mut [u8]) -> &'a str {
        let bytes = self.as_bytes();              // ① 拿到字节
        let len = bytes.len();                    // ② 字节长度
        buf[..len].copy_from_slice(bytes);        // ③ 复制进 buf
        core::str::from_utf8(&buf[..len]).unwrap() // ④ 转回 &str
    }
}

impl Tostr for i32 {
    fn to_str<'a>(&self, buf: &'a mut [u8]) -> &'a str {
        let mut idx = 0usize;

        // ① 处理负号
        if *self < 0 {
            buf[idx] = b'-';
            idx += 1;
        }
        let mut n: u32 = self.unsigned_abs();   // ② 取绝对值（u32 无溢出）

        // ③ 从低位到高位拆出每一位数字
        let mut digits = [0u8; 10];
        let mut len = 0;
        loop {
            digits[len] = (n % 10) as u8;   // 取最低位
            len += 1;
            n /= 10;                        // 去掉最低位
            if n == 0 { break; }
        }

        // ④ 倒序写入 buf（因为 digits 是低位在前）
        while len > 0 {
            len -= 1;
            buf[idx] = b'0' + digits[len];  // 数字 → 字符
            idx += 1;
        }

        core::str::from_utf8(&buf[..idx]).unwrap()
    }
}
/// 把 u32 写入 buf，返回新的写入位置
fn write_u32(mut n: u32, buf: &mut [u8], mut idx: usize) -> usize {
    let mut digits = [0u8; 10];
    let mut len = 0;
    loop {
        digits[len] = (n % 10) as u8;
        len += 1;
        n /= 10;
        if n == 0 { break; }
    }
    while len > 0 {
        len -= 1;
        buf[idx] = b'0' + digits[len];
        idx += 1;
    }
    idx
}

impl Tostr for f64 {
    fn to_str<'a>(&self, buf: &'a mut [u8]) -> &'a str {
        let mut idx = 0usize;

        // ① 负号
        if *self < 0.0 {
            buf[idx] = b'-';
            idx += 1;
        }
        let v = self.abs();

        // ② 拆整数部分 + 小数部分（保留 2 位）
        let mut int_part = v as i64;                    // 截断取整数
        // no_std 下没有 f64::round()，用「+0.5 再截断」等价实现四舍五入（v 已 abs 为非负）
        let mut frac = ((v - int_part as f64) * 100.0 + 0.5) as u32;

        // ③ 处理四舍五入进位（3.999 → 4.00）
        if frac >= 100 {
            frac = 0;
            int_part += 1;
        }

        // ④ 写整数部分
        idx = write_u32(int_part as u32, buf, idx);

        // ⑤ 写小数点 + 两位小数
        buf[idx] = b'.';
        idx += 1;
        buf[idx] = b'0' + (frac / 10) as u8;   // 十位
        idx += 1;
        buf[idx] = b'0' + (frac % 10) as u8;   // 个位
        idx += 1;

        core::str::from_utf8(&buf[..idx]).unwrap()
    }
}

impl Tostr for bool {
    fn to_str<'a>(&self, buf: &'a mut [u8]) -> &'a str {
        let s: &str = if *self { "true" } else { "false" };
        let bytes = s.as_bytes();
        buf[..bytes.len()].copy_from_slice(bytes);
        core::str::from_utf8(&buf[..bytes.len()]).unwrap()
    }
}

/// 一道题 = 3 个固定动作，依次显示到屏的 t0/t1/t2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    Q1, Q2, Q3, Q4, Q5, Q6,
}

impl Question {
    /// 条码 → 题目（⚠️ 扫码枪条码格式还没定，先占位，之后补真实条码）
    pub fn from_barcode(s: &str) -> Option<Self> {
        match s {
            "第1道题" => Some(Question::Q1),
            // TODO: 等扫码枪条码格式确定后，补 Q2~Q6 的真实条码
            _ => None,
        }
    }

    /// 题目 → 3 个动作（固定，填 t0/t1/t2）
    pub fn actions(self) -> [&'static str; 3] {
        match self {
            Question::Q1 => ["夹红色", "夹蓝色", "夹绿色"],
            Question::Q2 => ["占位2", "占位2", "占位2"],
            Question::Q3 => ["占位3", "占位3", "占位3"],
            Question::Q4 => ["占位4", "占位4", "占位4"],
            Question::Q5 => ["占位5", "占位5", "占位5"],
            Question::Q6 => ["占位6", "占位6", "占位6"],
        }
    }
}
