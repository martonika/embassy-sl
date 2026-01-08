//! Linked Direct Memory Access (LDMA) driver.
//!
//! This driver provides DMA transfer functionality for Silicon Labs EFR32 series MCUs.
//! LDMA enables efficient memory-to-memory and peripheral-to-memory transfers
//! without CPU intervention.
//!
//! # Features
//!
//! - Memory to memory transfers
//! - Peripheral to memory transfers (e.g., SPI, UART, ADC)
//! - Memory to peripheral transfers
//! - Linked descriptor chains for complex transfers
//! - Configurable transfer sizes (byte, half-word, word)
//!
//! # Example
//!
//! ```no_run,ignore
//! use embassy_silabs::ldma::{Ldma, Config, Transfer};
//!
//! // Initialize LDMA
//! let mut ldma = Ldma::new(config);
//!
//! // Simple memory-to-memory transfer
//! let src = [1u32, 2, 3, 4];
//! let mut dst = [0u32; 4];
//! ldma.transfer_blocking(0, &src, &mut dst).unwrap();
//! ```
#![warn(missing_docs)]

use core::future::poll_fn;
use core::sync::atomic::{AtomicU8, Ordering};
use core::task::Poll;

use embassy_sync::waitqueue::AtomicWaker;

use crate::chip::pac;

/// Number of DMA channels available.
pub const CHANNEL_COUNT: usize = 8;

/// Channel allocation state.
static CHANNEL_ALLOC: AtomicU8 = AtomicU8::new(0);

/// Wakers for each DMA channel.
static CHANNEL_WAKERS: [AtomicWaker; CHANNEL_COUNT] = [
    AtomicWaker::new(),
    AtomicWaker::new(),
    AtomicWaker::new(),
    AtomicWaker::new(),
    AtomicWaker::new(),
    AtomicWaker::new(),
    AtomicWaker::new(),
    AtomicWaker::new(),
];

/// LDMA transfer size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum TransferSize {
    /// Byte (8-bit) transfers.
    Byte = 0,
    /// Half-word (16-bit) transfers.
    HalfWord = 1,
    /// Word (32-bit) transfers.
    #[default]
    Word = 2,
}

/// Source address increment mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum SrcInc {
    /// Increment source address by one unit.
    #[default]
    One = 0,
    /// Increment source address by two units.
    Two = 1,
    /// Increment source address by four units.
    Four = 2,
    /// Do not increment source address.
    None = 3,
}

/// Destination address increment mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum DstInc {
    /// Increment destination address by one unit.
    #[default]
    One = 0,
    /// Increment destination address by two units.
    Two = 1,
    /// Increment destination address by four units.
    Four = 2,
    /// Do not increment destination address.
    None = 3,
}

/// Block size for arbitration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum BlockSize {
    /// One transfer per arbitration.
    Unit1 = 0,
    /// Two transfers per arbitration.
    Unit2 = 1,
    /// Three transfers per arbitration.
    Unit3 = 2,
    /// Four transfers per arbitration.
    #[default]
    Unit4 = 3,
    /// Six transfers per arbitration.
    Unit6 = 4,
    /// Eight transfers per arbitration.
    Unit8 = 5,
    /// 16 transfers per arbitration.
    Unit16 = 6,
    /// 32 transfers per arbitration.
    Unit32 = 7,
    /// 64 transfers per arbitration.
    Unit64 = 8,
    /// 128 transfers per arbitration.
    Unit128 = 9,
    /// 256 transfers per arbitration.
    Unit256 = 10,
    /// 512 transfers per arbitration.
    Unit512 = 11,
    /// 1024 transfers per arbitration.
    Unit1024 = 12,
    /// Lock arbitration during entire transfer.
    All = 15,
}

/// LDMA error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Error {
    /// No channels available.
    NoChannel,
    /// Invalid channel number.
    InvalidChannel,
    /// Transfer error (bus error or descriptor error).
    TransferError,
    /// Transfer count too large.
    CountTooLarge,
}

/// LDMA configuration.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct Config {
    /// Number of fixed priority channels (0-7).
    /// Channels 0 to numFixed-1 have fixed priority, rest use round-robin.
    pub num_fixed_priority: u8,
}

/// Transfer configuration for a single DMA transfer.
#[derive(Clone)]
#[non_exhaustive]
pub struct TransferConfig {
    /// Transfer size (byte, half-word, word).
    pub size: TransferSize,
    /// Source address increment.
    pub src_inc: SrcInc,
    /// Destination address increment.
    pub dst_inc: DstInc,
    /// Block size for arbitration.
    pub block_size: BlockSize,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            size: TransferSize::Word,
            src_inc: SrcInc::One,
            dst_inc: DstInc::One,
            block_size: BlockSize::Unit4,
        }
    }
}

/// DMA descriptor for linked transfers.
///
/// This structure must be aligned to 4 bytes and matches the hardware descriptor format.
#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct Descriptor {
    /// Control word.
    pub ctrl: u32,
    /// Source address.
    pub src: u32,
    /// Destination address.
    pub dst: u32,
    /// Link to next descriptor (or 0 for end).
    pub link: u32,
}

impl Descriptor {
    /// Create a memory-to-memory transfer descriptor.
    pub fn memory_to_memory(
        src: *const u8,
        dst: *mut u8,
        count: u16,
        config: &TransferConfig,
    ) -> Self {
        let ctrl = Self::build_ctrl(count, config, false);
        Self {
            ctrl,
            src: src as u32,
            dst: dst as u32,
            link: 0,
        }
    }

    /// Create a memory-to-peripheral transfer descriptor.
    pub fn memory_to_peripheral(
        src: *const u8,
        peripheral_reg: *mut u32,
        count: u16,
        config: &TransferConfig,
    ) -> Self {
        let mut cfg = config.clone();
        cfg.dst_inc = DstInc::None; // Peripheral register doesn't increment
        let ctrl = Self::build_ctrl(count, &cfg, false);
        Self {
            ctrl,
            src: src as u32,
            dst: peripheral_reg as u32,
            link: 0,
        }
    }

    /// Create a peripheral-to-memory transfer descriptor.
    pub fn peripheral_to_memory(
        peripheral_reg: *const u32,
        dst: *mut u8,
        count: u16,
        config: &TransferConfig,
    ) -> Self {
        let mut cfg = config.clone();
        cfg.src_inc = SrcInc::None; // Peripheral register doesn't increment
        let ctrl = Self::build_ctrl(count, &cfg, false);
        Self {
            ctrl,
            src: peripheral_reg as u32,
            dst: dst as u32,
            link: 0,
        }
    }

    /// Link this descriptor to another.
    pub fn link_to(&mut self, next: &Descriptor) {
        // Set LINK bit and absolute address
        self.link = (next as *const Descriptor as u32) | 1;
    }

    /// Build the control word for a descriptor.
    fn build_ctrl(count: u16, config: &TransferConfig, done_ifs: bool) -> u32 {
        let xfer_cnt = count.saturating_sub(1) as u32;

        // Structure type = transfer (0)
        // STRUCTREQ = 1 (trigger on structure load)
        // XFERCNT = count - 1
        // BLOCKSIZE = config.block_size
        // DONEIFSEN = done_ifs
        // REQMODE = 0 (block mode)
        // SRCINC = config.src_inc
        // SIZE = config.size
        // DSTINC = config.dst_inc
        // SRCMODE = 0 (absolute)
        // DSTMODE = 0 (absolute)

        (1 << 3)  // STRUCTREQ
            | ((xfer_cnt & 0x7FF) << 4)  // XFERCNT [14:4]
            | ((config.block_size as u32) << 15)  // BLOCKSIZE [18:15]
            | (if done_ifs { 1 << 21 } else { 0 })  // DONEIFSEN
            | ((config.src_inc as u32) << 26)  // SRCINC [27:26]
            | ((config.size as u32) << 28)  // SIZE [29:28]
            | ((config.dst_inc as u32) << 30)  // DSTINC [31:30]
    }
}

/// LDMA driver for managing DMA transfers.
pub struct Ldma {
    _private: (),
}

impl Ldma {
    /// Initialize the LDMA controller.
    ///
    /// This enables the LDMA clock and configures the controller.
    pub fn new(config: Config) -> Self {
        // Enable clocks for LDMA and LDMAXBAR
        pac::CMU.clken0().modify(|w| {
            w.set_ldma(true);
            w.set_ldmaxbar(true);
        });

        let ldma = ldma_regs();

        // Enable LDMA
        ldma.en().write(|w| w.set_en(true));

        // Configure control register
        ldma.ctrl().write(|w| {
            w.set_numfixed(config.num_fixed_priority);
        });

        // Disable all channels
        ldma.chdis().write(|w| w.0 = 0xFF);

        // Clear debug halt
        ldma.dbghalt().write(|w| w.0 = 0);

        // Clear request disable
        ldma.reqdis().write(|w| w.0 = 0);

        // Enable error interrupt
        ldma.ien().write(|w| w.set_error(true));

        // Clear all interrupt flags
        ldma.if_().write(|w| w.0 = 0xFFFF_FFFF);

        // Enable LDMA interrupt in NVIC
        unsafe {
            cortex_m::peripheral::NVIC::unmask(pac::Interrupt::LDMA);
        }

        Self { _private: () }
    }

    /// Allocate a DMA channel.
    ///
    /// Returns a channel handle if one is available.
    pub fn allocate_channel(&self) -> Option<Channel> {
        loop {
            let current = CHANNEL_ALLOC.load(Ordering::Acquire);
            for ch in 0..CHANNEL_COUNT {
                if current & (1 << ch) == 0 {
                    let new = current | (1 << ch);
                    if CHANNEL_ALLOC
                        .compare_exchange(current, new, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                    {
                        return Some(Channel { index: ch as u8 });
                    }
                    break; // Retry from beginning
                }
            }
            if current == 0xFF {
                return None; // All channels in use
            }
        }
    }

    /// Perform a blocking memory-to-memory transfer.
    pub fn transfer_blocking<T: Copy>(
        &self,
        channel: &Channel,
        src: &[T],
        dst: &mut [T],
    ) -> Result<(), Error> {
        if src.len() != dst.len() || src.is_empty() {
            return Err(Error::CountTooLarge);
        }

        let count = src.len();
        if count > 2048 {
            return Err(Error::CountTooLarge);
        }

        let size = match core::mem::size_of::<T>() {
            1 => TransferSize::Byte,
            2 => TransferSize::HalfWord,
            4 => TransferSize::Word,
            _ => return Err(Error::CountTooLarge),
        };

        let config = TransferConfig {
            size,
            ..Default::default()
        };

        let desc = Descriptor::memory_to_memory(
            src.as_ptr() as *const u8,
            dst.as_mut_ptr() as *mut u8,
            count as u16,
            &config,
        );

        self.start_transfer_with_descriptor(channel, &desc)?;
        self.wait_complete_blocking(channel)?;

        Ok(())
    }

    /// Perform an async memory-to-memory transfer.
    pub async fn transfer<T: Copy>(
        &self,
        channel: &Channel,
        src: &[T],
        dst: &mut [T],
    ) -> Result<(), Error> {
        if src.len() != dst.len() || src.is_empty() {
            return Err(Error::CountTooLarge);
        }

        let count = src.len();
        if count > 2048 {
            return Err(Error::CountTooLarge);
        }

        let size = match core::mem::size_of::<T>() {
            1 => TransferSize::Byte,
            2 => TransferSize::HalfWord,
            4 => TransferSize::Word,
            _ => return Err(Error::CountTooLarge),
        };

        let config = TransferConfig {
            size,
            ..Default::default()
        };

        let desc = Descriptor::memory_to_memory(
            src.as_ptr() as *const u8,
            dst.as_mut_ptr() as *mut u8,
            count as u16,
            &config,
        );

        self.start_transfer_with_descriptor(channel, &desc)?;
        self.wait_complete(channel).await?;

        Ok(())
    }

    /// Start a transfer using a descriptor.
    pub fn start_transfer_with_descriptor(
        &self,
        channel: &Channel,
        desc: &Descriptor,
    ) -> Result<(), Error> {
        let ldma = ldma_regs();
        let ch = channel.index as usize;

        // Clear done flag for this channel
        ldma.chdone().modify(|w| w.0 &= !(1 << ch));

        // Clear any pending interrupt for this channel
        ldma.if_().write(|w| w.0 = 1 << ch);

        // Enable interrupt for this channel
        ldma.ien().modify(|w| w.0 |= 1 << ch);

        // Set the link address to the descriptor
        set_channel_link(ch, desc as *const Descriptor as u32);

        // Set channel config (default settings)
        set_channel_cfg(ch, 0);

        // Start transfer by loading the descriptor
        ldma.linkload().write(|w| w.0 = 1 << ch);

        Ok(())
    }

    /// Wait for transfer to complete (blocking).
    fn wait_complete_blocking(&self, channel: &Channel) -> Result<(), Error> {
        let ldma = ldma_regs();
        let ch = channel.index as usize;
        let ch_mask = 1u32 << ch;

        loop {
            let if_flags = ldma.if_().read();

            // Check for error
            if if_flags.error() {
                ldma.if_().write(|w| w.set_error(true));
                return Err(Error::TransferError);
            }

            // Check if channel is done
            if ldma.chdone().read().0 & ch_mask != 0 {
                ldma.chdone().modify(|w| w.0 &= !ch_mask);
                ldma.if_().write(|w| w.0 = ch_mask);
                return Ok(());
            }
        }
    }

    /// Wait for transfer to complete (async).
    async fn wait_complete(&self, channel: &Channel) -> Result<(), Error> {
        let ch = channel.index as usize;
        let ch_mask = 1u32 << ch;

        poll_fn(|cx| {
            CHANNEL_WAKERS[ch].register(cx.waker());

            let ldma = ldma_regs();
            let if_flags = ldma.if_().read();

            // Check for error
            if if_flags.error() {
                ldma.if_().write(|w| w.set_error(true));
                return Poll::Ready(Err(Error::TransferError));
            }

            // Check if channel is done
            if ldma.chdone().read().0 & ch_mask != 0 {
                ldma.chdone().modify(|w| w.0 &= !ch_mask);
                ldma.if_().write(|w| w.0 = ch_mask);
                return Poll::Ready(Ok(()));
            }

            Poll::Pending
        })
        .await
    }

    /// Check if a channel transfer is complete.
    pub fn is_complete(&self, channel: &Channel) -> bool {
        let ldma = ldma_regs();
        let ch_mask = 1u32 << channel.index;
        ldma.chdone().read().0 & ch_mask != 0
    }

    /// Stop a DMA transfer on a channel.
    pub fn stop(&self, channel: &Channel) {
        let ldma = ldma_regs();
        let ch_mask = 1u32 << channel.index;

        // Disable the channel
        ldma.chdis().write(|w| w.0 = ch_mask);

        // Disable interrupt for this channel
        ldma.ien().modify(|w| w.0 &= !ch_mask);
    }

    /// Get the number of remaining transfer items for a channel.
    pub fn remaining_count(&self, channel: &Channel) -> u32 {
        let ldma = ldma_regs();
        let ch = channel.index as usize;

        // Check if done
        if ldma.chdone().read().0 & (1 << ch) != 0 {
            return 0;
        }

        // Read XFERCNT from channel CTRL register
        let ctrl = get_channel_ctrl(ch);
        let xfercnt = (ctrl >> 4) & 0x7FF;
        xfercnt + 1
    }
}

impl Drop for Ldma {
    fn drop(&mut self) {
        let ldma = ldma_regs();

        // Disable all channels
        ldma.chdis().write(|w| w.0 = 0xFF);

        // Disable interrupts
        ldma.ien().write(|w| w.0 = 0);

        // Disable LDMA
        ldma.en().write(|w| w.set_en(false));
    }
}

/// Handle for an allocated DMA channel.
pub struct Channel {
    index: u8,
}

impl Channel {
    /// Get the channel index.
    pub fn index(&self) -> u8 {
        self.index
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        // Free the channel
        CHANNEL_ALLOC.fetch_and(!(1 << self.index), Ordering::Release);
    }
}

/// LDMA interrupt handler.
///
/// Call this from your LDMA interrupt handler to process DMA completion events.
pub fn on_interrupt() {
    let ldma = ldma_regs();
    let if_flags = ldma.if_().read().0;

    // Clear all handled interrupt flags
    ldma.if_().write(|w| w.0 = if_flags);

    // Wake all channels that have completed
    for (ch, waker) in CHANNEL_WAKERS.iter().enumerate().take(CHANNEL_COUNT) {
        if if_flags & (1 << ch) != 0 {
            waker.wake();
        }
    }

    // Handle error interrupt
    if if_flags & (1 << 31) != 0 {
        // Error bit is bit 31
        for waker in &CHANNEL_WAKERS {
            waker.wake();
        }
    }
}

// ============================================================================
// Helper functions for register access
// ============================================================================

#[inline]
fn ldma_regs() -> pac::ldma::Ldma {
    #[cfg(feature = "_ns")]
    {
        pac::LDMA_NS
    }
    #[cfg(not(feature = "_ns"))]
    {
        pac::LDMA_S
    }
}

#[inline]
fn ldmaxbar_regs() -> pac::ldmaxbar::Ldmaxbar {
    #[cfg(feature = "_ns")]
    {
        pac::LDMAXBAR_NS
    }
    #[cfg(not(feature = "_ns"))]
    {
        pac::LDMAXBAR_S
    }
}

/// Set channel LINK register (descriptor address).
fn set_channel_link(ch: usize, addr: u32) {
    let ldma = ldma_regs();
    // Channel registers are at offset 0x5C + ch * 0x30
    // LINK is at offset +0x14 within each channel
    unsafe {
        let base = ldma.as_ptr() as *mut u32;
        let link_reg = base.add((0x5C + 0x14) / 4 + ch * (0x30 / 4));
        core::ptr::write_volatile(link_reg, addr & !0x3); // Clear lower 2 bits
    }
}

/// Set channel CFG register.
fn set_channel_cfg(ch: usize, cfg: u32) {
    let ldma = ldma_regs();
    unsafe {
        let base = ldma.as_ptr() as *mut u32;
        let cfg_reg = base.add(0x5C / 4 + ch * (0x30 / 4));
        core::ptr::write_volatile(cfg_reg, cfg);
    }
}

/// Get channel CTRL register value.
fn get_channel_ctrl(ch: usize) -> u32 {
    let ldma = ldma_regs();
    unsafe {
        let base = ldma.as_ptr() as *const u32;
        let ctrl_reg = base.add((0x5C + 0x08) / 4 + ch * (0x30 / 4));
        core::ptr::read_volatile(ctrl_reg)
    }
}

/// Peripheral request signals for LDMA transfers.
///
/// Use these with `configure_peripheral_channel` to configure DMA triggers.
/// Values are SOURCESEL | (SIGSEL << 4).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PeripheralSignal(pub u32);

impl PeripheralSignal {
    /// No peripheral signal (software triggered).
    pub const NONE: Self = Self(0);
    /// USART0 TX buffer level.
    pub const USART0_TXBL: Self = Self(0x0001_0400);
    /// USART0 RX data available.
    pub const USART0_RXDATAV: Self = Self(0x0000_0400);
    /// EUSART0 TX FIFO level.
    pub const EUSART0_TXFL: Self = Self(0x0010_0c00);
    /// EUSART0 RX FIFO level.
    pub const EUSART0_RXFL: Self = Self(0x0001_0c00);
    /// EUSART1 TX FIFO level.
    pub const EUSART1_TXFL: Self = Self(0x0010_0d00);
    /// EUSART1 RX FIFO level.
    pub const EUSART1_RXFL: Self = Self(0x0001_0d00);
    /// I2C0 TX buffer level.
    pub const I2C0_TXBL: Self = Self(0x0001_0500);
    /// I2C0 RX data available.
    pub const I2C0_RXDATAV: Self = Self(0x0000_0500);
    /// I2C1 TX buffer level.
    pub const I2C1_TXBL: Self = Self(0x0001_0600);
    /// I2C1 RX data available.
    pub const I2C1_RXDATAV: Self = Self(0x0000_0600);
    /// IADC0 scan result available.
    pub const IADC0_SCAN: Self = Self(0x0000_0800);
    /// IADC0 single result available.
    pub const IADC0_SINGLE: Self = Self(0x0001_0800);
    /// Timer0 overflow/underflow.
    pub const TIMER0_UFOF: Self = Self(0x0003_0000);
    /// Timer1 overflow/underflow.
    pub const TIMER1_UFOF: Self = Self(0x0003_0100);
    /// MSC write data.
    pub const MSC_WDATA: Self = Self(0x0000_0200);
}

impl Ldma {
    /// Configure a channel for peripheral-triggered transfers.
    ///
    /// This sets up the LDMAXBAR request select for the given channel.
    pub fn configure_peripheral_channel(&self, channel: &Channel, signal: PeripheralSignal) {
        let xbar = ldmaxbar_regs();
        let ch = channel.index as usize;

        // Set the request select for this channel
        unsafe {
            let base = xbar.as_ptr() as *mut u32;
            // REQSEL registers start at offset 0x10, each 4 bytes
            let reqsel_reg = base.add(0x10 / 4 + ch);
            core::ptr::write_volatile(reqsel_reg, signal.0);
        }
    }
}
