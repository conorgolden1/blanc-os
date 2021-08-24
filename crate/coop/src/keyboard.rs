//! The keyboard handler that handles the reading of scancodes asynchronously using the OS executor
use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use pc_keyboard::{DecodedKey, Keyboard, ScancodeSet1, layouts};
use core::{pin::Pin, task::{Poll, Context}};
use futures_util::{StreamExt, stream::Stream, task::AtomicWaker};


static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

use printer::{print, println};

pub fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get()  {
        if let Err(_) = queue.push(scancode)  {
            println!("WARNING: scancode queue full; dropping keyboard input");   
        } else {
            WAKER.wake();
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
            .expect("Keyboard ScancodeStream::new should only be called once");
        ScancodeStream {_private: ()}
    }
}


impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<u8>> {
         let queue = SCANCODE_QUEUE.try_get().expect("not initialized");
         
         if let Ok(scancode) = queue.pop() {
             return Poll::Ready(Some(scancode));
         }
         WAKER.register(&context.waker());
         match queue.pop() {
             Ok(scancode) => {
                 WAKER.take();
                 Poll::Ready(Some(scancode))
             }
             Err(crossbeam_queue::PopError) => Poll::Pending,
         }
    }
}


pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(layouts::Us104Key,
         ScancodeSet1, pc_keyboard::HandleControl::Ignore);

    while let Some(scancode) = scancodes.next().await  {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => print!("{}", character),
                    DecodedKey::RawKey(key) => print!("{:?}", key),
                }
            }
        }
    }
}