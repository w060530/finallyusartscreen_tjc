

const BUFFER_SIZE: usize = 256;
const MAX_FRAME_LEN: usize = 15; // 0xAA 帧头 + 最长 body 13 + 0xBB 帧尾

pub struct RingBuffer {
    buf: [u8; BUFFER_SIZE],
    read_index: usize,
    
    pub write_index: usize,
}

impl RingBuffer {
    pub const fn new() -> Self {
        Self {
            buf: [0u8; BUFFER_SIZE],
            read_index: 0,
            write_index: 0,
        }
    }

    
    pub fn add_read_index(&mut self, length: usize) {
        self.read_index += length;
        self.read_index %= BUFFER_SIZE;
    }

    
    pub fn read(&self, i: usize) -> u8 {
        let index = i % BUFFER_SIZE;
        self.buf[index]
    }

    
    pub fn get_length(&self) -> usize {
        (self.write_index + BUFFER_SIZE - self.read_index) % BUFFER_SIZE
    }

    
    pub fn get_remain(&self) -> usize {
        BUFFER_SIZE - self.get_length()
    }

    
    pub fn write(&mut self, data: &[u8]) -> usize {
        let length = data.len();
        
        if self.get_remain() < length {
            return 0;
        }
        // memcpy 等价：copy_from_slice
        let end = self.write_index + length;
        if end < BUFFER_SIZE {
            self.buf[self.write_index..end].copy_from_slice(data);
            self.write_index += length;
        } else {
            let first_length = BUFFER_SIZE - self.write_index;
            self.buf[self.write_index..].copy_from_slice(&data[..first_length]);
            self.buf[..length - first_length].copy_from_slice(&data[first_length..]);
            self.write_index = length - first_length;
        }
        length
    }

    
    pub fn get_command(&mut self, command_buf: &mut [u8]) -> usize {
        loop {
            // 至少要有帧头 + 帧尾 2 字节
            if self.get_length() < 2 {
                return 0;
            }
            // 找帧头 0xAA
            if self.read(self.read_index) != 0xAA {
                self.add_read_index(1);
                continue;
            }
            // 从帧头后一位开始找帧尾 0xBB，帧长不超过 MAX_FRAME_LEN
            let mut frame_len = 1;
            let mut found = false;
            while frame_len < MAX_FRAME_LEN && frame_len < self.get_length() {
                if self.read(self.read_index + frame_len) == 0xBB {
                    found = true;
                    frame_len += 1;
                    break;
                }
                frame_len += 1;
            }
            if !found {
                // 数据不够就先等；若已超最大帧长，说明这个 0xAA 是假帧头，跳过继续找
                if self.get_length() >= MAX_FRAME_LEN {
                    self.add_read_index(1);
                    continue;
                }
                return 0;
            }
            // 拷出完整帧（含 0xAA 帧头、0xBB 帧尾）
            for i in 0..frame_len {
                command_buf[i] = self.read(self.read_index + i);
            }
            self.add_read_index(frame_len);
            return frame_len;
        }
    }
    pub fn update_write_index(&mut self, dma_remaining: usize) {
        self.write_index = (BUFFER_SIZE - dma_remaining) % BUFFER_SIZE;
    }
}
