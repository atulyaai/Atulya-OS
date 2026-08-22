//! Raw Limine boot-protocol requests (no external crate).
//!
//! This is a self-contained re-implementation of the Limine v6 request structs
//! (mirroring `limine` crate 0.6) so the kernel doesn't depend on that crate,
//! which currently declares `#![feature(ptr_metadata)]` and won't compile on
//! recent nightlies. All struct layouts & magic IDs come from the Limine spec.
//!
//! The bootloader scans the ELF's loaded sections for `.requests_start` /
//! `.requests` / `.requests_end` and honours the request structs placed there.

#![allow(non_camel_case_types, dead_code)]

use core::cell::UnsafeCell;

/// Common request magic, always first in every request.
pub const COMMON_MAGIC: [u64; 2] = [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b];

#[repr(C)]
pub struct RequestsStartMarker([u64; 4]);
impl RequestsStartMarker {
    pub const fn new() -> Self {
        Self([
            0xf6b8f4b39de7d1ae,
            0xfab91a6940fcb9cf,
            0x785c6ed015d3e316,
            0x181e920a7852b9d9,
        ])
    }
}

#[repr(C)]
pub struct RequestsEndMarker([u64; 2]);
impl RequestsEndMarker {
    pub const fn new() -> Self {
        Self([0xadc0e0531bb10d03, 0x9572709f31764c62])
    }
}

/// Request the Limine base-revision. The bootloader zeroes revision[1] to
/// signal support; `is_supported()` checks that.
#[repr(C)]
pub struct BaseRevision {
    magic: UnsafeCell<[u64; 3]>,
}
impl BaseRevision {
    pub const MAX_SUPPORTED: u64 = 6;
    pub const fn new() -> Self {
        Self {
            magic: UnsafeCell::new([
                0xf9562b2d5c95a6c8,
                0x6a7b384944536bdc,
                Self::MAX_SUPPORTED,
            ]),
        }
    }
    pub fn is_supported(&self) -> bool {
        unsafe { (self.magic.get() as *const u64).add(2).read_volatile() == 0 }
    }
}
unsafe impl Send for BaseRevision {}
unsafe impl Sync for BaseRevision {}

#[repr(C)]
pub struct Request<Resp, Req = ()> {
    magic: [u64; 2],
    id: [u64; 2],
    revision: u64,
    response: UnsafeCell<*mut Response<Resp>>,
    request: Req,
}

impl<Resp, Req> Request<Resp, Req> {
    pub const unsafe fn new_raw(id: [u64; 2], revision: u64, request: Req) -> Self {
        Self {
            magic: COMMON_MAGIC,
            id,
            revision,
            response: UnsafeCell::new(core::ptr::null_mut()),
            request,
        }
    }
    pub fn response(&self) -> Option<&'static Response<Resp>> {
        let p = unsafe { self.response.get().read_volatile() };
        if p.is_null() {
            None
        } else {
            Some(unsafe { &*p })
        }
    }
}
unsafe impl<Resp, Req> Send for Request<Resp, Req> {}
unsafe impl<Resp, Req> Sync for Request<Resp, Req> {}

#[repr(C)]
pub struct Response<T> {
    pub revision: u64,
    pub data: T,
}
impl<T> Response<T> {
    pub fn data(&self) -> &T {
        &self.data
    }
}
unsafe impl<T> Send for Response<T> {}
unsafe impl<T> Sync for Response<T> {}

/* ===== Framebuffer ===== */
#[repr(C)]
pub struct Framebuffer {
    address: *mut (),
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bpp: u16,
    pub memory_model: u8,
    _resvd0: [u8; 7],
    _edid_size: u64,
    _edid: *const (),
}
impl Framebuffer {
    pub fn address(&self) -> *mut () {
        self.address
    }
    pub fn size(&self) -> usize {
        (self.height * self.pitch) as usize
    }
}

pub struct FramebufferRespData {
    framebuffer_count: u64,
    framebuffers: *const (),
}
impl FramebufferRespData {
    pub fn framebuffers(&self) -> &[&Framebuffer] {
        unsafe {
            &*core::ptr::slice_from_raw_parts(
                self.framebuffers as *const &Framebuffer,
                self.framebuffer_count as usize,
            )
        }
    }
}
pub type FramebufferRequest = Request<FramebufferRespData>;
impl FramebufferRequest {
    pub const fn new() -> Self {
        unsafe { Self::new_raw([0x9d5827dcd881dd75, 0xa3148604f6fab11b], 0, ()) }
    }
}

/* ===== Memory map ===== */
#[repr(C)]
pub struct MemmapEntry {
    pub base: u64,
    pub length: u64,
    pub type_: u64,
}
pub const MEMMAP_USABLE: u64 = 0;

pub struct MemmapRespData {
    entry_count: u64,
    entries: *const (),
}
impl MemmapRespData {
    pub fn entries(&self) -> &[&MemmapEntry] {
        unsafe {
            &*core::ptr::slice_from_raw_parts(
                self.entries as *const &MemmapEntry,
                self.entry_count as usize,
            )
        }
    }
}
pub type MemmapRequest = Request<MemmapRespData>;
impl MemmapRequest {
    pub const fn new() -> Self {
        unsafe { Self::new_raw([0x67cf3d9d378a806f, 0xe304acdfc50c3c62], 0, ()) }
    }
}

/* ===== HHDM (direct-map offset for phys->virt access) ===== */
pub struct HhdmRespData {
    pub offset: u64,
}
pub type HhdmRequest = Request<HhdmRespData>;
impl HhdmRequest {
    pub const fn new() -> Self {
        unsafe { Self::new_raw([0x48dcf1cb8ad2b852, 0x63984e959a98244b], 0, ()) }
    }
}

/* ===== Paging mode ===== */
#[repr(u64)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PagingMode {
    X86_64_4LVL = 0,
    X86_64_5LVL = 1,
}
impl PagingMode {
    pub const MIN: Self = PagingMode::X86_64_4LVL;
    pub const MAX: Self = PagingMode::X86_64_5LVL;
}
pub struct PagingModeReqData {
    pub mode: PagingMode,
    pub max_mode: PagingMode,
    pub min_mode: PagingMode,
}
pub struct PagingModeRespData {
    pub mode: PagingMode,
}
pub type PagingModeRequest = Request<PagingModeRespData, PagingModeReqData>;
impl PagingModeRequest {
    pub const PREFER_MAXIMUM: Self = Self::new(PagingMode::MAX, PagingMode::MAX, PagingMode::MIN);
    pub const fn new(mode: PagingMode, max_mode: PagingMode, min_mode: PagingMode) -> Self {
        Self::new_with(mode, max_mode, min_mode)
    }
    const fn new_with(mode: PagingMode, max_mode: PagingMode, min_mode: PagingMode) -> Self {
        unsafe {
            Self::new_raw(
                [0x95c1a0edab0944cb, 0xa4e5cb3842f7488a],
                0,
                PagingModeReqData { mode, max_mode, min_mode },
            )
        }
    }
}
