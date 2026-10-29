use std::{ffi::c_void, mem};

use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::Memory::{
    VirtualQueryEx, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE,
    PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY
};
use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
use windows::core::Owned;

#[derive(Clone, Copy)]
pub struct Pattern<'a> {
    pattern: &'a [u8],
    mask: &'a [u8]
}

impl<'a> Pattern<'a> {
    pub const fn new(pattern: &'a [u8], mask: &'a [u8]) -> Self {
        if pattern.len() != mask.len() { panic!("pattern and mask must have the same length") };

        Self { pattern, mask }
    }

    pub const fn len(&self) -> usize {
        self.pattern.len()
    }

    fn do_match(&self, bytes: &[u8]) -> bool {
        if bytes.len() != self.len() { return false };

        self.pattern
            .iter()
            .zip(self.mask)
            .zip(bytes)
            .all(|((p, m), b)| p & m == b & m)
    }

    pub fn find(&self, buf: &[u8]) -> Option<usize> {
        if self.len() == 0 { return None };
        buf.windows(self.len()).position(|w| self.do_match(w))
    }

    pub fn matches(&self, buf: &'a [u8]) -> Matches<'a> {
        Matches { pattern: *self, buf, pos: 0 }
    }
}

pub struct Matches<'a> {
    pattern: Pattern<'a>,
    buf: &'a [u8],
    pos: usize
}

impl Iterator for Matches<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let rest = self.buf.get(self.pos..)?;
        let i = self.pattern.find(rest)?;
        let at = self.pos + i;

        self.pos = at + 1;

        Some(at)
    }
}

pub const fn get_pattern_len(pattern: &str) -> usize {
    let bytes = pattern.as_bytes();
    
    let mut i = 0;
    let mut count = 0;
    while i < bytes.len() {
        let b = bytes[i];
        
        if !b.is_ascii() { panic!("pattern must be ASCII") };
        if !b.is_ascii_whitespace() {
            match b {
                b'?' | b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' => count += 1,
                _ => panic!("pattern byte must be a hex digit, '?', or whitespace")
            }
        }

        i += 1;
    }

    if count % 2 != 0 { panic!("pattern must have an even number of hex/'?' nibbles") };
    
    count / 2
}

pub const fn parse_pattern<const N: usize>(pattern: &str) -> ([u8; N], [u8; N]) {
    let bytes = pattern.as_bytes();

    let mut pattern = [0; N];
    let mut mask = [0; N];

    let mut acc = None;
    let mut i = 0;
    let mut j = 0;
    while i < bytes.len() {
        let b = bytes[i];

        if !b.is_ascii() { panic!("pattern must be ASCII") };
        if !b.is_ascii_whitespace() {
            match b {
                b'?' => match acc.take() {
                    Some((p, m)) => {
                        pattern[j] = p;
                        mask[j] = m;
                        
                        j += 1;
                    },
                    None => acc = Some((0, 0))
                },
                b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' => {
                    let digit = (b as char).to_digit(16).unwrap() as u8;

                    match acc.take() {
                        Some((mut p, mut m)) => {
                            p |= digit;
                            pattern[j] = p;

                            m |= 0xf;
                            mask[j] = m;

                            j += 1;
                        },
                        None => acc = Some((digit << 4, 0xf0))
                    }
                },
                _ => panic!("pattern byte must be a hex digit, '?', or whitespace")
            }
        }

        i += 1;
    }

    if acc.is_some() { panic!("pattern has a trailing unpaired nibble") };

    (pattern, mask)
}

#[macro_export]
macro_rules! pattern {
    ($s:expr) => {
        {
            const N: usize = $crate::pattern::get_pattern_len($s);
            const PARSED: ([u8; N], [u8; N]) = $crate::pattern::parse_pattern::<N>($s);
            $crate::Pattern::new(&PARSED.0, &PARSED.1)
        }
    };
}

pub struct MemoryRegion {
    pub base: usize,
    pub size: usize
}

pub struct Process<'a> {
    handle: &'a Owned<HANDLE>
}

impl<'a> Process<'a> {
    pub unsafe fn new(handle: &'a Owned<HANDLE>) -> Self {
        Self { handle }
    }

    pub fn regions(&self) -> windows::core::Result<Vec<MemoryRegion>> {
        let mut info = SYSTEM_INFO::default();
        unsafe { GetSystemInfo(&mut info) };

        let max = info.lpMaximumApplicationAddress as usize;

        let mut regions = Vec::new();
        let mut addr = info.lpMinimumApplicationAddress as usize;
        while addr < max {
            let mut memory_info = MEMORY_BASIC_INFORMATION::default();
            unsafe {
                let n = VirtualQueryEx(**self.handle, Some(addr as *const c_void), &mut memory_info, mem::size_of::<MEMORY_BASIC_INFORMATION>());
                if n == 0 { return Err(windows::core::Error::from_thread()) };
            }

            let base = memory_info.BaseAddress as usize;

            if let PAGE_EXECUTE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY = memory_info.Protect {
                regions.push(MemoryRegion { base, size: memory_info.RegionSize });
            }

            match base.checked_add(memory_info.RegionSize) {
                Some(next) if next > addr => addr = next,
                _ => break
            }
        }

        Ok(regions)
    }

    pub fn read(&self, addr: usize, buf: &mut [u8]) -> windows::core::Result<usize> {
        let mut read = 0;
        unsafe {
            ReadProcessMemory(**self.handle, addr as *const c_void, buf.as_mut_ptr().cast(), buf.len(), Some(&mut read))?;
        }

        Ok(read)
    }

    pub fn scan(&self, pattern: Pattern, chunk_size: usize) -> windows::core::Result<Vec<usize>> {
        let mut results = Vec::new();

        if pattern.len() == 0 || chunk_size == 0 {
            return Ok(results);
        }

        let mut buf = vec![0; chunk_size + pattern.len() - 1];
        
        for region in self.regions()? {
            let mut pos = 0;
            let mut tail_len = 0;
            while pos < region.size {
                let addr = region.base + pos;
                let want = (region.size - pos).min(buf.len() - tail_len);
                let read = match self.read(addr, &mut buf[tail_len..tail_len + want]) {
                    Ok(r) if r == 0 => break,
                    Ok(r) => r,
                    _ => {
                        pos += want;
                        tail_len = 0;
                        continue;
                    }
                };

                let valid = tail_len + read;

                for m in pattern.matches(&buf[..valid]) {
                    results.push(addr - tail_len + m);
                }

                tail_len = (pattern.len() - 1).min(valid);
                buf.copy_within(valid - tail_len..valid, 0);
                pos += read;
            }
        }

        Ok(results)
    }
}