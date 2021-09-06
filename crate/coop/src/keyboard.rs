//! The keyboard handler that handles the reading of scancodes asynchronously using the OS executor
use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{stream::Stream, task::AtomicWaker, StreamExt};
use pc_keyboard::{layouts, DecodedKey, Keyboard, ScancodeSet1};
use printer::{print, println};

/// A queue used for pulling keystrokes asynchronously
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

/// A primitive waker used for waking the asynchronous poll_next task in the scancode stream
static WAKER: AtomicWaker = AtomicWaker::new();

/// Add a scancode to the scancode queue and wake the scancode stream
pub fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if queue.push(scancode).is_err() {
            println!("WARNING: scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();
        }
    } else {
        println!("WARNING: Keyboard scancode queue uninitialized");
    }
}

/// Wrapper struct for implementing the stream trait for the scancode queue
/// for asynchronous functionality
pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    /// Create a new scancode stream with a queue size of 100, can only be
    /// initialized once else will panic with respective message
    fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("Keyboard ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;
    /// Poll for the next scancode in the stream, if the queue isn't empty else
    /// register this context with a waker and continue execution once woken.
    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<u8>> {
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

impl Default for ScancodeStream {
    fn default() -> Self {
        Self::new()
    }
}

/// Print each keypress added to the scancode queue asynchronously from the pushed
/// PIC scancodes
pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        layouts::Us104Key,
        ScancodeSet1,
        pc_keyboard::HandleControl::Ignore,
    );

    while let Some(scancode) = scancodes.next().await {
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
