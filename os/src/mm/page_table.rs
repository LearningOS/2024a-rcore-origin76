//! Implementation of [`PageTableEntry`] and [`PageTable`].

use core::mem;
use core::ptr::copy_nonoverlapping;

use super::{frame_alloc, FrameTracker, MapPermission, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use crate::config::PAGE_SIZE;
use crate::syscall::process::{TaskInfo, TimeVal};
use crate::task::{current_user_token, map_current_memory_set, outer_get_pcb, TaskStatus};
use crate::timer::*;
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

bitflags! {
    /// page table entry flags
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// page table entry structure
pub struct PageTableEntry {
    /// bits of page table entry
    pub bits: usize,
}

impl PageTableEntry {
    /// Create a new page table entry
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    /// Create an empty page table entry
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    /// Get the physical page number from the page table entry
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    /// Get the flags from the page table entry
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    /// The page pointered by page table entry is valid?
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is readable?
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is writable?
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is executable?
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// page table structure
pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    /// Create a new page table
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    /// Find PageTableEntry by VirtPageNum, create a frame for a 4KB page table if not exist
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    /// Find PageTableEntry by VirtPageNum
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }
    /// set the map between virtual page number and physical page number
    #[allow(unused)]
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }
    /// remove the map between virtual page number and physical page number
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }
    /// get the page table entry from the virtual page number
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    /// get the token from the page table
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

/// Translate&Copy a ptr[u8] array with LENGTH len to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

/// write timeval in kernel
pub fn write_time_val(token: usize, ptr: *mut TimeVal) {
    let page_table = PageTable::from_token(token);

    let sec_ptr = unsafe { &(*ptr).sec as *const usize };
    let sec_va = VirtAddr::from(sec_ptr as usize);
    let sec_vpn = sec_va.floor();

    let sec_ppn = page_table.translate(sec_vpn).unwrap().ppn();
    let sec_offset = sec_va.page_offset();
    let sec_bytes = &mut sec_ppn.get_bytes_array()[sec_offset..sec_offset + 8];

    let usec_ptr = unsafe { &(*ptr).usec as *const usize };
    let usec_va = VirtAddr::from(usec_ptr as usize);
    let usec_vpn = usec_va.floor();

    let usec_ppn = page_table.translate(usec_vpn).unwrap().ppn();
    let usec_offset = usec_va.page_offset();
    let usec_bytes = &mut usec_ppn.get_bytes_array()[usec_offset..usec_offset + 8];

    let time_us = get_time_us();
    let us = time_us % 1_000_000;
    let sec = time_us / 1_000_000;

    unsafe {
        let s_ptr = sec_bytes.as_mut_ptr() as *mut usize;
        *s_ptr = sec;
        let us_ptr = usec_bytes.as_mut_ptr() as *mut usize;
        *us_ptr = us;
    };
}

/// write task_info in kernel
pub fn write_task_info(token: usize, ptr: *mut TaskInfo) {
    // Taskinfo offset : [4*500] 8 1

    let pcb = outer_get_pcb();
    unsafe {
        let temp = TaskInfo {
            status: TaskStatus::Running,
            syscall_times: (*pcb).syscall_count,
            time: get_time_ms() - (*pcb).start_time as usize,
        };
        let info: *const TaskInfo = &temp as *const TaskInfo;
        let p: *mut u8 = info as *mut u8;
        let mut bytes = translated_byte_buffer(token, ptr as *const u8, mem::size_of::<TaskInfo>());
        let mut cur = 0;
        for byte in &mut bytes {
            if cur + byte.len() <= mem::size_of::<TaskInfo>() {
                copy_nonoverlapping(p.add(cur), (byte).as_mut_ptr(), byte.len());
            }
            cur += byte.len()
        }
    }
}

/// mmap
pub fn mmap_impl(start: usize, len: usize, port: usize) -> isize {
    let page_table = PageTable::from_token(current_user_token());
    if VirtAddr::from(start).page_offset() != 0 {
        return -1;
    }
    let flags = MapPermission::U
        | if port & (1 << 0) != 0 {
            MapPermission::R
        } else {
            MapPermission::empty()
        }
        | if port & (1 << 1) != 0 {
            MapPermission::W
        } else {
            MapPermission::empty()
        }
        | if port & (1 << 2) != 0 {
            MapPermission::X
        } else {
            MapPermission::empty()
        };

    let mut idx = 0;
    while idx < len {
        let pte = page_table.find_pte(VirtAddr::from(start + idx).floor());
        if pte.is_some() && pte.unwrap().is_valid() {
            return -1;
        }
        idx += PAGE_SIZE;
    }
    map_current_memory_set(VirtAddr::from(start) , VirtAddr::from(start+len), flags)
}

/// unmap
pub fn munmap_impl(start: usize, len: usize) -> isize {
    let mut page_table = PageTable::from_token(current_user_token());
    if VirtAddr::from(start).page_offset() != 0 {
        return -1;
    }

    let mut idx = 0;
    while idx < len {
        let pte = page_table.find_pte(VirtAddr::from(start + idx).floor());
        println!(
            "find {} {}",
            VirtAddr::from(start + idx).floor().0,
            pte.is_some()
        );
        if !pte.is_some() || !pte.unwrap().is_valid(){
            return -1;
        }
        idx += PAGE_SIZE;
    }

    idx = 0;
    while idx < len {
        println!("unmap {}", VirtAddr::from(start + idx).floor().0);
        page_table.unmap(VirtAddr::from(start + idx).floor());
        idx += PAGE_SIZE;
    }

    return 0;
}
