use core::slice;

use printer::{println, print};

global_asm!(include_str!("syscall_interrupts.s"));

extern "C" {
    pub fn syscall(call_num : u64, param1 : u64, param2 : u64, param3: u64) -> u64;
    fn exit_syscall(call_num : u64) -> !;
}


pub(crate) static SYSTEM_CALLS : [fn(param1 : u64, param2 : u64, param3: u64); 1] = [
        print
    ];

fn print(_file_descriptor : u64, affective_address : u64, bytes: u64) {
    // TODO IMPLEMENT FD AND ERROR CHECKING
    unsafe {
        let slice = slice::from_raw_parts(affective_address as *const _, bytes as usize);
        let str = core::str::from_utf8(slice).unwrap();
        print!("{}", str);
    }
    


}

