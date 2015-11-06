//
//  SOS: the Stupid Operating System
//  by Hawk Weisman (hi@hawkweisman.me)
//
//  Copyright (c) 2015 Hawk Weisman
//  Released under the terms of the MIT license. See `LICENSE` in the root
//  directory of this repository for more information.
//
//! 64-bit Interrupt Descriptor Table implementation.
//!
//! Refer to section 6.10 of the _Intel® 64 and IA-32 Architectures
//! Software Developer’s Manual_ for more information.
use core::mem;
use spin::Mutex;

extern {
    /// Offset of the 64-bit GDT main code segment.
    /// Exported by `boot.asm`
    static gdt64_offset: u16;
}

const IDT_ENTRIES: usize = 256;
type Handler = unsafe extern "C" fn() -> ();

/// An IDT entry is called a gate.
///
/// Based on code from the OS Dev Wiki
/// http://wiki.osdev.org/Interrupt_Descriptor_Table#Structure
#[repr(C, packed)]
struct Gate { /// bits 0 - 15 of the offset
              offset_lower: u16
            , /// code segment selector (GDT or LDT)
              selector: u16
            , /// always zero
              zero: u8
            , /// indicates the gate's type and attributes.
              /// the second half indicates the type:
              ///   + `0b1100`: Call gate
              ///   + `0b1110`: Interrupt gate
              ///   + `0b1111`: Trap Gate
              type_attr: u8
            , /// bits 16 - 31 of the offset
              offest_mid: u16
            , /// bits 32 - 63 of the offset
              offset_upper: u32
            , /// always zero (according to the spec, this is "reserved")
              reserved: u32
            }

impl Gate {
    /// Creates a new IDT gate marked as `absent`.
    ///
    /// This is basically just for filling the new IDT table
    /// with valid (but useless) gates upon init.
    const fn absent() -> Self {
        Gate { offset_lower: 0
             , selector: 0
             , zero: 0
             , type_attr: 0b0000_1110
             , offset_mid: 0
             , offset_upper: 0
             , reserved: 0
        }
    }

    /// Creates a new IDT gate pointing at the given handler function.
    fn new(handler: Handler) -> Gate {
        unsafe { // trust me on this.
                 // `mem::transmute()` is glorious black magic
            let (low, mid, high): (u16, u16, u32)
                = mem::transmute(handler)

            Gate { offset_lower: low
                 , selector: gdt64_offset
                 , zero: 0
                 , type_attr: 0b1000_1110
                 , offset_mid: mid
                 , offset_upper: high
                 , reserved: 0
                 }
        }
    }
}


struct Idt([Gate; IDT_ENTRIES]);

impl Idt {
    /// Get the IDT pointer struct to pass to `lidt`
    fn get_ptr(&self) -> IdtPtr {
        IdtPtr { limit: mem::size_of::<Gate>() * IDT_ENTRIES
               , base: &self.0[0] as *const Gate
               }
    }

    /// Enable interrupts
    unsafe fn enable_interrupts() {
        asm!("sti" :::: "volatile")
    }

    /// This is just a wrapper for prettiness reasons.
    #[inline]
    unsafe fn load(&self) {
        self.get_ptr()
            .load()
    }
}

/// This is the format that `lidt` expects for the pointer to the IDT.
/// ...apparently.
#[repr(C, packed)]
struct IdtPtr { limit: u16
              , base: u64
              }

impl IdtPtr {
    /// Load the IDT at the given location.
    /// This just calls `lidt`.
    pub unsafe fn load(&self) {
        asm!(  "lidt ($0)"
            :: "{rax}"(self)
            :: "volatile" );
    }
}

/// Global Interrupt Descriptor Table instance
/// Our global IDT.
static IDT: Mutex<Idt>
    = Mutex::new(Idt([Gate::absent(); IDT_ENTRIES]));