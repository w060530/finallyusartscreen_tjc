


/// 屏上的一个控件
/// C = context 缓冲区大小（如 128）
pub struct Object<'a, const T: usize> {
    pub name: &'a str,
    pub context: [u8; T],
    pub len: usize,
}

impl<'a, const T: usize> Object<'a, T> {
    pub fn new(name: &'a str, context: &[u8]) -> Self {
    let mut obj =    Object{name,
            context:[0u8;T],
            len:0,

        };
        obj.set_context(context);
        obj

    }

    pub fn set_context(&mut self, context: &[u8]) {
        let len  = context.len().min(T) ;
        self.context[..len].copy_from_slice(&context[..len]);
        self.len =len;
    }
}

/// 串口发送抽象
/// N = 控件个数，T = 每个控件的 context 大小
pub trait SerialHandle<const N: usize, const T: usize> {
    fn refresh(&mut self, objs: &[Object<'_,T>; N]) -> Result<(), ()>;
}

/// 单控件刷新抽象：只发一个控件，不涉及控件个数 N（否则调用时 N 无法推断）。
/// 刷新哪个控件由传入的 obj 决定；T 只是每个控件的缓冲大小。
pub trait RefreshOne<const T: usize> {
    fn refresh_one(&mut self, obj: &Object<'_, T>) -> Result<(), ()>;
}

/// 串口屏：N 个控件 + 一个串口句柄
pub struct Screen<'a, const N: usize, const T: usize, S: SerialHandle<N, T>> {
    pub serial: S,
    pub objects: [Object<'a, T>; N],
}


    
pub fn build_cmd (name:&str,context:&[u8],len:usize,buf:&mut [u8])-> usize {

    let mut i = 0 ;
    buf[i..i+name.len()].copy_from_slice(name.as_bytes());
    i += name.len();
    buf[i] = b'=';
    i+=1;
    buf[i] = b'"';
    i += 1;
    buf[i..i + len].copy_from_slice(&context[..len]);
    i += len;
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

// pub struct MockSerial{
//     pub buf:[u8;256],
//     pub len:usize,
// }

// impl <const N:usize,const T: usize> SerialHandle<N,T>for MockSerial{
//     fn refresh(&mut self,objs:&[Object<'_,T>;N])->Result<(),()>{
//         let mut pos =0 ;
//         for obj in objs.iter()  {
//             pos+= build_cmd(obj.name,&obj. context,obj. len,&mut  self.buf[pos..]);
       
//         }
//         self.len = pos;
//         Ok(())

//     }
    
// }