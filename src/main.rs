extern crate libc;

use libc::_SC_PAGESIZE;
use std::mem;

// Can implement Index[Mut] in order to conveniently access the buffer.
//
struct JitMemory {
    buffer: *mut u8,
}

impl JitMemory {
    pub fn new(buffer_size: usize) -> Self {
        let buffer = Self::allocate(buffer_size, 0xC3);
        Self { buffer }
    }

    pub fn write(&mut self, source: &[u8]) {
        unsafe { self.buffer.copy_from(source.as_ptr(), source.len()) }
    }

    pub fn run<T>(&self) -> T {
        let entry_point: fn() -> T = unsafe { mem::transmute(self.buffer) };
        entry_point()
    }

    fn allocate(buffer_size: usize, fill_value: u8) -> *mut u8 {
        let page_size = Self::page_size();

        // The rules are a bit more complex (see https://linux.die.net/man/3/posix_memalign).
        //
        assert!(
            buffer_size % page_size == 0,
            "buffer_size not multiple of page size"
        );

        unsafe {
            // Since posix_memalign discards the existing associate memory, we can use init as null
            // pointer.
            // We can use MaybeUninit, but in this case it's more verbose and no more useful.
            //
            let mut buffer_addr = std::ptr::null_mut();

            // Allocate the memory, aligned to the page.
            //
            libc::posix_memalign(&mut buffer_addr, page_size, buffer_size);

            // Set the permissions.
            //
            libc::mprotect(
                buffer_addr,
                buffer_size,
                libc::PROT_EXEC | libc::PROT_READ | libc::PROT_WRITE,
            );

            // Note that the API accepts a 32 bits value (c_int), however it converts it to an unsigned
            // char.
            //
            libc::memset(buffer_addr, fill_value as i32, buffer_size);

            buffer_addr as *mut u8
        }
    }

    fn page_size() -> usize {
        unsafe { libc::sysconf(_SC_PAGESIZE) as usize }
    }
}

// A cleaned up version of https://www.jonathanturner.org/building-a-simple-jit-in-rust.
//
fn main() {
    let mut jit: JitMemory = JitMemory::new(4096);

    // mov RAX, 0x3
    jit.write(&[0x48, 0xc7, 0xc0, 0x3, 0, 0, 0]);

    let result = jit.run::<i64>();

    println!("{}", result);
}
