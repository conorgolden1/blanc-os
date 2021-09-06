//! TODO DOCUMENT
use core::sync::atomic::{AtomicU8, Ordering};
use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures_util::{Stream, StreamExt};
use futures_util::task::AtomicWaker;
use printer::{WRITER, print, println};
use x86_64::instructions::port::{PortGeneric, ReadWriteAccess};
use x86_64::instructions::port::Port;
use core::{pin::Pin, task::{Poll, Context}};

static mut PORT_64 : PortGeneric<u32, ReadWriteAccess> = Port::new(0x64);
static mut PORT_60 : PortGeneric<u32, ReadWriteAccess> = Port::new(0x60);
static mut STATE : AtomicU8 = AtomicU8::new(0);
static WAKER: AtomicWaker = AtomicWaker::new();
static mut MOUSEPACKET : [u8; 3] = [0; 3];

static SCANCODE_QUEUE: OnceCell<ArrayQueue<[u8; 3]>> = OnceCell::uninit();

/// # Safety
pub unsafe fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get()  {
        match STATE.fetch_add(1, Ordering::Relaxed) {
            0 =>  {
                if scancode & 0b00001000 == 0b00001000 {
                    MOUSEPACKET[0] = scancode;
                } else {
                    STATE.swap(0, Ordering::Relaxed);
                }
            }, 
            1 => MOUSEPACKET[1] = scancode,
            2 => {
                MOUSEPACKET[2] = scancode;
                if queue.push(MOUSEPACKET).is_err()  {
                    println!("WARNING: scancode queue full; dropping mouse input");   
                } else {
                    WAKER.wake();
                }
                STATE.swap(0, Ordering::Relaxed);
            }
            _ => unreachable!("Mouse state should not exceed 2 in current impl")
    
        }
    } else {
        // println!("WARNING: scancode queue uninitialized");
    }
    
}

pub struct ScancodeStream {
    _private: (),
}


impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE.try_init_once(|| ArrayQueue::new(100))
            .expect("Mouse ScancodeStream::new should only be called once");
        ScancodeStream {_private: ()}
    }
}

impl Default for ScancodeStream {
    fn default() -> Self {
    Self::new()
    }
}

impl Stream for ScancodeStream {
    type Item = [u8; 3];

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<[u8; 3]>> {
         let queue = SCANCODE_QUEUE.try_get().expect("not initialized");
         
         if let Ok(scancode) = queue.pop() {
             return Poll::Ready(Some(scancode));
         }
         WAKER.register(context.waker());
         match queue.pop() {
             Ok(scancode) => {
                 WAKER.take();
                 Poll::Ready(Some(scancode))
             }
             Err(crossbeam_queue::PopError) => Poll::Pending,
         }
    }
}

pub async fn print_mouse() {
    let mut scancodes = ScancodeStream::new();
    let mut mouse_point  = MousePoint::new();
    init_mouse().unwrap();
    while let Some(scancode) = scancodes.next().await  {
        // println!("{}", WRITER.get().unwrap().lock().info.horizontal_resolution);
        // println!("{}", WRITER.get().unwrap().lock().info.vertical_resolution);
        
        
        let flags = MousePacketFlags::from_bits_truncate(scancode[0]);
        
        if !flags.contains(MousePacketFlags::XSIGN) {
            mouse_point.x += scancode[1] as i16;
            if flags.contains(MousePacketFlags::XOVERFLOW) {
                mouse_point.x += 255;
            }
        } else {

            mouse_point.x -= 256 - scancode[1] as i16;
            if flags.contains(MousePacketFlags::XOVERFLOW) {
                mouse_point.x -= 255;
            }
        }

        if !flags.contains(MousePacketFlags::YSIGN) {
            mouse_point.y -= scancode[2] as i16;
            if flags.contains(MousePacketFlags::YOVERFLOW) {
                mouse_point.y -= 255;
            }
        } else {
            mouse_point.y += 256 - scancode[2] as i16;
            if flags.contains(MousePacketFlags::YOVERFLOW) {
                mouse_point.y += 255;
            }
        }

        if mouse_point.x < 0 { mouse_point.x = 0}
        if mouse_point.x > WRITER.get().unwrap().lock().info.horizontal_resolution as i16 - 1{
             mouse_point.x = WRITER.get().unwrap().lock().info.horizontal_resolution as i16 - 1
        }
        
        if mouse_point.y < 0 { mouse_point.y = 0}
        if mouse_point.y > WRITER.get().unwrap().lock().info.vertical_resolution as i16 - 1{
             mouse_point.y = WRITER.get().unwrap().lock().info.vertical_resolution as i16 - 1
        }

        WRITER.get().unwrap().lock().write_pixel(mouse_point.x as usize,  mouse_point.y as usize, 255)

    }
}



use bitflags::bitflags;

bitflags! {
    /// First mouse byte bitflags
    /// 
    /// See [Byte 1](https://wiki.osdev.org/Mouse_Input#Initializing_a_PS2_Mouse)
    pub struct MousePacketFlags: u8 {
        ///Pages may not be accessed.
        const LEFTBUTTON = 1;

        ///Pages may be read.
        const RIGHTBUTTON = 1 << 1;

        ///Pages may be written.
        const MIDDLEBUTTON = 1 << 2;

        ///Pages may be executed.
        const XSIGN = 1 << 4;

        const YSIGN = 1 << 5;

        const XOVERFLOW = 1 << 6;

        const YOVERFLOW = 1 << 7;
    }
}

struct MousePoint {
    pub x : i16,
    pub y : i16,
}

impl MousePoint {
    pub fn new() -> Self {
        Self {
            x : 0,
            y : 0,
        }
    }
}










fn init_mouse() -> Result<(), &'static str> {
    
    unsafe { PORT_64.write(0xA8) } ;
    mouse_wait();
    unsafe { PORT_64.write(0x20) } ;
    mouse_wait_input();
    let mut status : u8 = unsafe { PORT_60.read() } as u8;
    status |= 0b10;
    mouse_wait();
    unsafe { PORT_64.write(0x60) } ;
    mouse_wait();
    unsafe { PORT_60.write(status as u32) } ;
    mouse_write(0xF6);
    mouse_read();
    mouse_write(0xF4);
    mouse_read();

    Ok(())
    
}


fn mouse_wait() {
    let mut timeout = 100000;
    while timeout != 0 {
        if unsafe { PORT_64.read() } & 0b10 == 0 {
            return
        }
        timeout -= 1;
    }
}

fn mouse_wait_input() {
    let mut timeout = 100000;
    while timeout != 0 {
        if unsafe { PORT_64.read() } & 0b1 == 0b1 {
            return
        }
        timeout -= 1;
    }
    
}

fn mouse_write(value : u8)  {
    mouse_wait();
    unsafe { PORT_64.write(0xD4) } ;
    mouse_wait();
    unsafe { PORT_60.write(value as u32) } ;
}

fn mouse_read() -> u32 {
    mouse_wait_input();
    unsafe { PORT_60.read() } 
    
}