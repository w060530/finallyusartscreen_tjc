
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
        let mut frac = ((v - int_part as f64) * 100.0).round() as u32;

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

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]

// pub enum Question {
//     Q1, Q2, Q3, Q4, Q5, Q6,
// }

// impl Question{
//      pub fn from_str(s: impl Tostr) -> Option<Self> {
//     let mut buf = [0u8; 64];
//     let s: &str = s.to_str(&mut buf);

//     match s {
//         "第1道题" => Some(Question::Q1),
//         "第2道题" => Some(Question::Q2),
//         "第3道题" => Some(Question::Q3),
//         "第4道题" => Some(Question::Q4),
//         "第5道题" => Some(Question::Q5),
//         "第6道题" => Some(Question::Q6),
//         _ => None,
//     }
// }


    
    
    
//     pub fn code(self) -> &'static str {
//     match self {
//         Question::Q1 => "01",
//         Question::Q2 => "02",
//         Question::Q3 => "03",
//         Question::Q4 => "04",
//         Question::Q5 => "05",
//         Question::Q6 => "06",
//     }
// }
// }

