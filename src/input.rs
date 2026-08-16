pub struct InputBuf {
    buf: [u8; 256],
    len: usize,
}

impl InputBuf {
    pub const fn new() -> Self {
    InputBuf {
        buf: [0u8; 256],   // 缓冲区全填 0
        len: 0,            // 还没写入任何字节
    }
}

    /// 逐字节喂入，凑满一行（\n）返回 Some，否则 None。自动 trim 换行和空白
    pub fn feed(&mut self, byte: u8) -> Option<&str> {
    // 遇到换行符，说明一行结束
    if byte == b'\n' {
        // ① 去掉末尾的 \r（Windows 换行是 \r\n）
        let mut end = self.len;
        if end > 0 && self.buf[end - 1] == b'\r' {
            end -= 1;
        }

        // ② 去掉首尾空格
        let mut start = 0;
        while start < end && self.buf[start] == b' ' {
            start += 1;
        }
        while end > start && self.buf[end - 1] == b' ' {
            end -= 1;
        }

        // ③ 把有效部分转成 &str
        let line = core::str::from_utf8(&self.buf[start..end]).ok()?;

        // ④ 重置缓冲区，准备收下一行
        self.len = 0;

        Some(line)
    } else {
        // 普通字节，存进缓冲区
        self.buf[self.len] = byte;
        self.len += 1;
        None
    }
}

}
