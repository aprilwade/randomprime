#![no_std]
// #![allow(unused)]

extern crate alloc;

use core::cmp;
use core::marker::PhantomData;
use core::mem::MaybeUninit;


#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum EXIChannel
{
    Chan0 = 0, Chan1 =1, Chan2 = 2,
}


#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum EXIDevice
{
    Dev0 = 0, Dev1 =1, Dev2 = 2,
}


#[repr(u32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum EXIMode
{
    Read = 0, Write = 1, _ReadWrite = 2,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum EXIFreq
{
    _1MHz = 0,
    _2MHz = 1,
    _4MHz = 2,
    _8MHz = 3,
    _16MHz = 4,
    Speed32MHz = 5,
}

// TODO: Primitives for disabling/restoring interrupts

type EXICallback = extern fn(chn: i32, dev: i32) -> i32;
extern {
    // fn EXIProbeEx(chn: EXIChannel) -> i32;
    fn EXILock(chn: EXIChannel, dev: EXIDevice, unlock_cb: Option<EXICallback>) -> i32;
    fn EXIUnlock(chn: EXIChannel) -> i32;
    // fn EXIImm(chn: EXIChannel, data: *mut u8, len: u32, mode: EXIMode, tc_cb: Option<EXICallback>) -> i32;
    fn EXIImmEx(chn: EXIChannel, data: *mut u8, len: u32, mode: EXIMode) -> i32;
    fn EXIDma(chn: EXIChannel, data: *mut u8, len: u32, mode: EXIMode, tc_cb: Option<EXICallback>) -> i32;
    fn EXISync(chn: EXIChannel) -> i32;
    fn EXISetExiCallback(chn: EXIChannel, cb: Option<EXICallback>) -> Option<EXICallback>;

    fn EXISelect(chn: EXIChannel, device: EXIDevice, speed: EXIFreq) -> i32;
    fn EXIDeselect(chn: EXIChannel) -> i32;

    fn OSDisableInterrupts() -> u32;
    fn OSRestoreInterrupts(enable: u32);
}

struct DisableInterrupts(bool);

impl DisableInterrupts
{
    fn new() -> DisableInterrupts
    {
        DisableInterrupts(unsafe { OSDisableInterrupts() } == 1)
    }
}

impl Drop for DisableInterrupts
{
    fn drop(&mut self)
    {
        unsafe {
            OSRestoreInterrupts(self.0 as u32);
        }
    }
}

struct ExiLockedDevice
{
    channel: EXIChannel,
    device: EXIDevice
}

impl ExiLockedDevice
{
    fn lock(channel: EXIChannel, device: EXIDevice) -> Option<ExiLockedDevice>
    {
        if unsafe { EXILock(channel, device, None) } == 0 {
            None
        } else {
            Some(ExiLockedDevice { channel, device })
        }
    }

    fn select(&mut self, freq: EXIFreq) -> ExiSelectedDevice
    {
        unsafe {
            EXISelect(self.channel, self.device, freq);
        }
        ExiSelectedDevice {
            channel: self.channel,
            device: self.device,
            freq,
            pd: PhantomData,
        }
    }
}

impl Drop for ExiLockedDevice
{
    fn drop(&mut self)
    {
        unsafe {
            EXIUnlock(self.channel);
        }
    }
}

struct ExiSelectedDevice<'a>
{
    device: EXIDevice,
    channel: EXIChannel,
    freq: EXIFreq,
    pd: PhantomData<&'a mut ExiLockedDevice>,
}

impl<'a> ExiSelectedDevice<'a>
{
    pub fn reselect(&mut self)
    {
        unsafe {
            EXIDeselect(self.channel);
            EXISelect(self.channel, self.device, self.freq);
        }
    }

    pub fn imm_read(&mut self, buf: &mut [MaybeUninit<u8>])
    {
        unsafe {
            EXIImmEx(self.channel, buf.as_mut_ptr() as *mut u8, buf.len() as u32, EXIMode::Read);
        }
    }

    pub fn imm_write(&mut self, buf: &[u8])
    {
        unsafe {
            EXIImmEx(self.channel, buf.as_ptr() as *mut u8, buf.len() as u32, EXIMode::Write);
        }
    }

    pub fn dma_read(&mut self, buf: &mut [MaybeUninit<u8>])
    {
        unsafe {
            EXIDma(self.channel, buf.as_mut_ptr() as *mut u8, buf.len() as u32, EXIMode::Read, None);
            EXISync(self.channel);
        }
    }

    pub fn dma_write(&mut self, buf: &[u8])
    {
        unsafe {
            EXIDma(self.channel, buf.as_ptr() as *mut u8, buf.len() as u32, EXIMode::Write, None);
            EXISync(self.channel);
        }
    }
}

impl<'a> Drop for ExiSelectedDevice<'a>
{
    fn drop(&mut self)
    {
        unsafe {
            EXIDeselect(self.channel);
        }
    }
}

trait ToBeBytes
{
    type Output: AsRef<[u8]>;
    fn to_be_bytes(self) -> Self::Output;
}

impl ToBeBytes for u16
{
    type Output = [u8; 2];
    fn to_be_bytes(self) -> Self::Output
    {
        self.to_be_bytes()
    }
}
impl ToBeBytes for u32
{
    type Output = [u8; 4];
    fn to_be_bytes(self) -> Self::Output
    {
        self.to_be_bytes()
    }
}

// TODO: RegisterName
trait BbaRegister
{
    type Output: ToBeBytes;
    fn to_read_addr(self) -> Self::Output;
    fn to_write_addr(self) -> Self::Output;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum BbaReg
{
    NCRA,// = 0x00,
    NCRB,// = 0x01,

    // LTPS = 0x04,
    LRPS,// = 0x05,

    IMR,// = 0x08,
    IR,// = 0x09,

    BP,// = 0x0a,
    // TLBP = 0x0c,
    // TWP = 0x0e,
    // IOB = 0x10,
    // TRP = 0x12,
    RWP,// = 0x16,
    RRP,// = 0x18,
    RHBP,// = 0x1a,

    // RXINTT = 0x14,

    NAFR_PAR0,// = 0x20,
    // NAFR_PAR1 = 0x21,
    // NAFR_PAR2 = 0x22,
    // NAFR_PAR3 = 0x23,
    // NAFR_PAR4 = 0x24,
    // NAFR_PAR5 = 0x25,
    // NAFR_MAR0 = 0x26,
    // NAFR_MAR1 = 0x27,
    // NAFR_MAR2 = 0x28,
    // NAFR_MAR3 = 0x29,
    // NAFR_MAR4 = 0x2a,
    // NAFR_MAR5 = 0x2b,
    // NAFR_MAR6 = 0x2c,
    // NAFR_MAR7 = 0x2d,

    // NWAYC = 0x30,
    // NWAYS = 0x31,

    // GCA = 0x32,

    // MISC = 0x3d,

    // TXFIFOCNT = 0x3e,
    WRTXFIFOD,// = 0x48,

    MISC2,// = 0x50,

    PacketBuf(u8, u8),

    // SI_ACTRL = 0x5c,
    // SI_STATUS = 0x5d,
    // SI_ACTRL2 = 0x60
}

impl BbaReg
{
    fn into_u32(self) -> u32
    {
        match self {
            BbaReg::NCRA => 0x00,
            BbaReg::NCRB => 0x01,
            BbaReg::LRPS => 0x05,
            BbaReg::IMR => 0x08,
            BbaReg::IR => 0x09,
            BbaReg::BP => 0x0a,
            BbaReg::RWP => 0x16,
            BbaReg::RRP => 0x18,
            BbaReg::RHBP => 0x1a,
            BbaReg::NAFR_PAR0 => 0x20,
            BbaReg::WRTXFIFOD => 0x48,
            BbaReg::MISC2 => 0x50,
            BbaReg::PacketBuf(u, l) => (u as u32) << 8 + l as u32,
        }
    }
}

impl BbaRegister for BbaReg
{
    type Output = u32;
    fn to_read_addr(self) -> u32
    {
        (self.into_u32() << 8) | 0x80000000
    }
    fn to_write_addr(self) -> u32
    {
        (self.into_u32() << 8) | 0xC0000000
    }
}


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum BbaCmdReg
{
    ExiId = 0x0,
    RevisionId,
    InterruptMask,
    Interrupt,
    DeviceId,
    AcStart,
    // HashReaD = 8,
    // HashWrite,
    // HashStatus = 0xb,
    // Reset = 0xf
}


// TODO: This should be u16...
impl BbaRegister for BbaCmdReg
{
    type Output = u16;
    fn to_read_addr(self) -> u16
    {
        ((self as u32) << 8) as u16
    }
    fn to_write_addr(self) -> u16
    {
        (((self as u32) << 8) | 0x4000) as u16
    }
}


struct BbaExiDevice<'a>(&'a mut ExiLockedDevice);

impl<'a> BbaExiDevice<'a>
{
    fn read_imm<I>(&mut self, reg: I, buf: &mut [MaybeUninit<u8>])
        where I: ToBeBytes
    {
        let reg_bytes = reg.to_be_bytes();
        let mut selected = self.0.select(EXIFreq::Speed32MHz);
        selected.imm_write(reg_bytes.as_ref());
        selected.imm_read(buf);
    }

    fn read_dma(&mut self, reg: BbaReg, buf: &mut [MaybeUninit<u8>])
    {
        let reg_bytes = reg.to_read_addr().to_be_bytes();
        let mut selected = self.0.select(EXIFreq::Speed32MHz);
        selected.imm_write(&reg_bytes[..]);
        selected.dma_read(buf);
    }

    fn read_u8<R>(&mut self, reg: R) -> u8
        where R: BbaRegister
    {
        let mut b = MaybeUninit::uninit();
        self.read_imm(reg.to_read_addr(), core::slice::from_mut(&mut b));
        unsafe { b.assume_init() }
    }

    fn read_u16<R>(&mut self, reg: R) -> u16
        where R: BbaRegister
    {
        let mut buf = [MaybeUninit::uninit(); 2];
        self.read_imm(reg.to_read_addr(), &mut buf[..]);
        unsafe { u16::from_be_bytes(core::mem::transmute(buf)) }
    }

    fn read_u32<R>(&mut self, reg: R) -> u32
        where R: BbaRegister
    {
        let mut buf = [MaybeUninit::uninit(); 4];
        self.read_imm(reg.to_read_addr(), &mut buf[..]);
        unsafe { u32::from_be_bytes(core::mem::transmute(buf)) }
    }

    fn write_imm<I>(&mut self, reg: I, buf: &[u8])
        where I: ToBeBytes
    {
        let reg_bytes = reg.to_be_bytes();
        let mut selected = self.0.select(EXIFreq::Speed32MHz);
        selected.imm_write(reg_bytes.as_ref());
        selected.imm_write(buf);
    }

    fn write_dma(&mut self, reg: BbaReg, buf: &[u8])
    {
        let reg_bytes = reg.to_write_addr().to_be_bytes();
        let mut selected = self.0.select(EXIFreq::Speed32MHz);
        selected.imm_write(&reg_bytes[..]);
        selected.dma_write(buf);
    }

    fn write_u8<R>(&mut self, reg: R, val: u8)
        where R: BbaRegister
    {
        self.write_imm(reg.to_write_addr(), core::slice::from_ref(&val));
    }

    fn write_u16<R>(&mut self, reg: R, val: u16)
        where R: BbaRegister
    {
        self.write_imm(reg.to_write_addr(), &val.to_be_bytes()[..]);
    }

    fn write_u32<R>(&mut self, reg: R, val: u32)
        where R: BbaRegister
    {
        self.write_imm(reg.to_write_addr(), &val.to_be_bytes()[..]);
    }
}


use alloc::boxed::Box;
use smoltcp::phy::DeviceCapabilities;
use smoltcp::time::Instant;

struct PacketRingBuffer
{
    buf: Box<[MaybeUninit<u8>]>,
    read_offset: usize,
    write_offset: usize,
}

unsafe fn assume_init_slice<T>(slice: &[MaybeUninit<T>]) -> &[T]
{
    &*(slice as *const [_] as *const [T])
}

unsafe fn assume_init_slice_mut<T>(slice: &mut [MaybeUninit<T>]) -> &mut [T]
{
    &mut *(slice as *mut [_] as *mut [T])
}

fn uninit_slice<T>(slice: &[T]) -> &[MaybeUninit<T>]
{
    unsafe {
        &*(slice as *const [T] as *const [MaybeUninit<T>])
    }
}

impl PacketRingBuffer
{
    fn new(buf: Box<[MaybeUninit<u8>]>) -> Self
    {
        PacketRingBuffer {
            buf,
            read_offset: 0,
            write_offset: 0,
        }
    }
    // XXX How much of this should be done with interrupts disabled?
    //     If it's only ever called from the interrupt handler, then it doesn't matter...
    fn enqueue_with(&mut self, len: usize, f: impl FnOnce(&mut [MaybeUninit<u8>]))
        -> bool
    {
        // Add room to place the length at the start of the ring buffer slot
        let len = len + 2;

        let mut buf_start = self.write_offset;
        if buf_start > self.read_offset && buf_start + len > self.buf.len() {
            // Write a length of zero to ensure we know to skip to the start of the buffer
            // (So we can always have a contiguous buffer to hand to smoltcp)
            if self.buf.len() - buf_start >= 2 {
                self.buf[buf_start..buf_start + 2].copy_from_slice(&[MaybeUninit::new(0); 2]);
            }
            buf_start = 0;
        }
        if buf_start < self.read_offset && buf_start + len > self.read_offset {
            return false;
        }
        f(&mut self.buf[buf_start + 2..buf_start + len]);
        let len_bytes = (len as u16).to_be_bytes();
        self.buf[buf_start..buf_start + 2].copy_from_slice(uninit_slice(&len_bytes[..]));
        self.write_offset = buf_start + len;
        true
    }

    fn can_dequeue(&self) -> bool
    {
        let _di = DisableInterrupts::new();
        self.write_offset != self.read_offset
    }

    fn dequeue_with(&mut self, f: impl FnOnce(&mut [u8]))
        -> bool
    {
        let mut read_offset = {
            // This might be overly conservative, but just to be safe, we only access/update
            // self.read_offset and self.write_offset with interrupts disabled
            let _di = DisableInterrupts::new();
            if self.write_offset == self.read_offset {
                return false;
            }
            self.read_offset
        };
        if self.buf.len() - read_offset < 4 {
            read_offset = 0;
        }
        let len = loop {
            let len = u16::from_be_bytes(unsafe { [
                self.buf[read_offset].assume_init(),
                self.buf[read_offset + 1].assume_init(),
            ] });
            if len == 0 {
                read_offset = 0;
            } else {
                break len as usize;
            }
        };
        let buf = &mut self.buf[read_offset + 2..read_offset + len];
        f(unsafe { assume_init_slice_mut(buf) });

        {
            let _di = DisableInterrupts::new();
            self.read_offset = read_offset + len;
        }

        true
    }
}

struct BbaState
{
    rx_ring_buf: PacketRingBuffer,
    iface: smoltcp::iface::EthernetInterface<'static, 'static, 'static, BbaEthernetDevice>,
}

static mut BBA_STATE: Option<BbaState> = None;

struct BbaEthernetDevice;
impl<'a> smoltcp::phy::Device<'a> for BbaEthernetDevice {
    type RxToken = BbaRxToken<'a>;
    type TxToken = BbaTxToken<'a>;
    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)>
    {
        if unsafe { BBA_STATE.as_mut().unwrap() }.rx_ring_buf.can_dequeue() {
            Some((BbaRxToken(PhantomData), BbaTxToken(PhantomData)))
        } else {
            None
        }
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken>
    {
        Some(BbaTxToken(PhantomData))
    }

    fn capabilities(&self) -> DeviceCapabilities
    {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1500;
        // XXX This isn't true really. We can burst an unlimited number of Txs
        caps.max_burst_size = Some(1);
        caps
    }
}


struct BbaRxToken<'a>(PhantomData<&'a mut BbaEthernetDevice>);
impl<'a> smoltcp::phy::RxToken for BbaRxToken<'a>
{
    fn consume<R, F>(self, _timestamp: Instant, f: F) -> smoltcp::Result<R>
        where F: FnOnce(&mut [u8]) -> smoltcp::Result<R>
    {
        let mut res = Err(smoltcp::Error::Exhausted);
        let res_ref = &mut res;

        unsafe { BBA_STATE.as_mut().unwrap() }.rx_ring_buf.dequeue_with(move |buf| {
            *res_ref = f(buf);
        });

        res
    }
}

struct BbaTxToken<'a>(PhantomData<&'a mut BbaEthernetDevice>);
impl<'a> smoltcp::phy::TxToken for BbaTxToken<'a>
{
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> smoltcp::Result<R>
        where F: FnOnce(&mut [u8]) -> smoltcp::Result<R>,
    {
        let mut buf = [0; 1518];
        let buf = &mut buf[..len];
        let r = f(buf)?;

        // Can this ever fail? TODO: Consider Err(smoltcp::Error::Exhausted)?
        let mut locked = ExiLockedDevice::lock(EXIChannel::Chan0, EXIDevice::Dev2).unwrap();
        let mut device = BbaExiDevice(&mut locked);

        // It appears that Dolphin immediately and synchronously performs the send, so we can
        // simply perform the send without any buffering.
        // Likewise, Dolphin doesn't care about alignment for the DMA operation, so don't bother

        // TODO: Is is actually necessary to disable interrupts here?
        {
            let _interrupts_disabled = DisableInterrupts::new();
            device.write_dma(BbaReg::WRTXFIFOD, buf);
        }
        {
            let _interrupts_disabled = DisableInterrupts::new();
            let ncra = device.read_u8(BbaReg::NCRA);
            // Set ST1 and unset ST0
            device.write_u8(BbaReg::NCRA, (ncra & !2) | 4);
        }

        Ok(r)
    }
}

// TODO: It might be cool to rate limit this somehow. Like only allow the interrupt to be fired 10
//       times every frame. Network QOS is extremely unimportant, but we want limit framerate
//       hitches as much as possible.
extern fn bba_interrupt_handler(chn: i32, dev: i32) -> i32
{
    if chn != 0 || dev != 2 {
        // TODO: Error?
        return 0;
    }
    // TODO: used cmd interrupt mask to temporarily neuter all interrupts for the bba?
    let mut locked = if let Some(l) = ExiLockedDevice::lock(EXIChannel::Chan0, EXIDevice::Dev2) {
        l
    } else {
        return 0;
    };

    let mut device = BbaExiDevice(&mut locked);
    // let ir = device.read_u8(BbaReg::IR);
    // if ir & 0x02 != 0 {
    //     // If we received a packet...
    // }

    let mut rrp = device.read_u8(BbaReg::RRP);
    let rwp = device.read_u8(BbaReg::RWP);

    // TODO: ligogc only allows the exception handler to dequeue a max of 32 packets in a given
    //       interrupt. It might be a good idea to implement a similiar limit
    while rrp != rwp {
        let descr = device.read_u32(BbaReg::PacketBuf(rrp, 0));
        let descr = Descriptor::from_bytes(descr.to_be_bytes());
        let page_count = (descr.packet_length >> 8) as u8;

        // Remove 4 to discard the descriptor
        let packet_length = descr.packet_length as usize - 4;
        let first_read_page_count = cmp::min(rrp + page_count, 0x10) - rrp;
        let first_read_len = cmp::min((first_read_page_count as usize) << 8, packet_length);

        let ring_buf = unsafe { &mut BBA_STATE.as_mut().unwrap().rx_ring_buf };
        let mut new_rrp = rrp;
        let did_enqueue = ring_buf.enqueue_with(packet_length, |buf| {
            device.read_dma(BbaReg::PacketBuf(rrp, 4), &mut buf[..first_read_len]);

            new_rrp += page_count;
            if first_read_len != packet_length {
                device.read_dma(BbaReg::PacketBuf(1, 0), &mut buf[first_read_len..packet_length]);
                // Second part read
                new_rrp = (new_rrp & 0x0f) + 1;
            }
        });
        if did_enqueue {
            // Only advance rrp if we had enough space in the ring buffer to store this packet
            rrp = new_rrp;
        } else {
            // If we didn't have enough space, leave the packet in the bba's memory and wait until
            // we receive another interrupt to try again
            break
        }
    }
    1
}

struct Descriptor
{
    _next_page: u8,
    _status: u8,
    packet_length: u16,

}
impl Descriptor
{
    fn from_bytes(bytes: [u8; 4]) -> Self
    {
        let i = u32::from_le_bytes(bytes);
        Descriptor {
            _next_page: (i & 0xff) as u8,
            _status: ((i >> 24) & 0xff) as u8,
            packet_length: ((i >> 12) & 0xfff) as u16,
        }
    }
}

use smoltcp::iface::{Neighbor, NeighborCache};
use smoltcp::wire::IpAddress;
static mut NEIGHBOR_CACHE: [Option<(IpAddress, Neighbor)>; 10] = [None; 10];

pub fn init()
{
    // TODO Only call this once
    let prev_cb = unsafe { EXISetExiCallback(EXIChannel::Chan2, Some(bba_interrupt_handler)) };
    assert_eq!(prev_cb, None);

    // TODO: Verify Dolphin is actually configured with the BBA enabled and fail if not
    //       (it might be possible for a user of this library to report that error to the user)

    let mut locked = ExiLockedDevice::lock(EXIChannel::Chan0, EXIDevice::Dev2).unwrap();
    let mut device = BbaExiDevice(&mut locked);

    // Initiate a device reset
    device.write_u8(BbaReg::NCRA, 0x1);
    // Read out the MAC address
    let mut mac_addr = [MaybeUninit::uninit(); 6];
    device.read_imm(BbaReg::NAFR_PAR0.to_read_addr(), &mut mac_addr[..]);
    let mac_addr = unsafe { assume_init_slice(&mac_addr[..]) };

    let iface = smoltcp::iface::EthernetInterfaceBuilder::new(BbaEthernetDevice)
        .ethernet_addr(smoltcp::wire::EthernetAddress::from_bytes(mac_addr))
        // TODO: Set a low limit. A defalut of 1024 seems huge given our memory budget
        //       Maybe using a static slice would be better anyway since it uses a small fixed
        //       amount of memory?
        .neighbor_cache(NeighborCache::new(unsafe { &mut NEIGHBOR_CACHE[..] }))
        .finalize();

    let buf = alloc::vec![MaybeUninit::uninit(); 1024 * 10].into_boxed_slice();
    unsafe {
        BBA_STATE = Some(BbaState {
            iface,
            rx_ring_buf: PacketRingBuffer::new(buf),
        });
    }

    // Setup receive ring buffer registers
    // In theory rrp, rwp, rhbp are 12-bit values, but we'll never see a value > 0x10 in any of
    // them, so we can treat them like 8-bit values instead
    // XXX If BP is 0x00 we will trample important register values
    device.write_u8(BbaReg::BP, 0x01);
    device.write_u8(BbaReg::RRP, 0x01);
    device.write_u8(BbaReg::RWP, 0x01);
    // TODO: Why does ogc use 0xf here? Dolphin's source suggests this is exlusive, not inclusive
    device.write_u8(BbaReg::RHBP, 0x10);

    // Automatically recover from full RX buffer (AUTORCVR)
    device.write_u8(BbaReg::MISC2, 0xf0);
    // Accept broadcast, but not multicast ethernet frames
    device.write_u8(BbaReg::NCRB, 0x10);
    // Only fire interrupts when a packet has been received
    device.write_u8(BbaReg::IMR, 0x2);

    // TODO: Start receiving packets
}

