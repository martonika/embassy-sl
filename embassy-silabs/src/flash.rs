//! Flash memory controller (MSC) driver.
//!
//! This driver provides flash read, write, and erase functionality for Silicon Labs
//! EFR32 series MCUs. It implements the `embedded-storage` traits for compatibility
//! with the embedded Rust ecosystem.
//!
//! # Features
//!
//! - Page erase
//! - Word-aligned writes
//! - Flash read
//! - Flash region info queries
//!
//! # Example
//!
//! ```no_run,ignore
//! use embassy_silabs::flash::Flash;
//!
//! let mut flash = Flash::new();
//!
//! // Erase a page (must be page-aligned)
//! flash.erase(0x0002_0000, 0x2000).unwrap();
//!
//! // Write data (must be word-aligned)
//! let data = [0x12345678u32, 0xDEADBEEF];
//! flash.write(0x0002_0000, bytemuck::bytes_of(&data)).unwrap();
//!
//! // Read data
//! let mut buf = [0u8; 8];
//! flash.read(0x0002_0000, &mut buf).unwrap();
//! ```
#![warn(missing_docs)]

use crate::chip::pac;

/// Flash page size in bytes.
/// For EFR32xG24, the page size is 8 KB (0x2000).
pub const PAGE_SIZE: u32 = 0x2000;

/// Flash base address.
pub const FLASH_BASE: u32 = 0x0800_0000;

/// Flash size in bytes.
/// For EFR32MG24B220F1536IM48, flash size is 1536 KB.
pub const FLASH_SIZE: u32 = 1536 * 1024;

/// MSC program timeout (number of polling iterations).
const PROGRAM_TIMEOUT: u32 = 10_000_000;

/// Flash error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Error {
    /// Operation tried to access an invalid address.
    InvalidAddress,
    /// The flash region is locked and cannot be written or erased.
    Locked,
    /// Operation timed out.
    Timeout,
    /// Address or size is not properly aligned.
    Alignment,
}

/// Flash memory controller driver.
pub struct Flash {
    _private: (),
}

impl Flash {
    /// Create a new Flash driver instance.
    ///
    /// This enables the MSC clock and initializes the controller.
    pub fn new() -> Self {
        // Enable MSC clock
        pac::CMU.clken1().modify(|w| {
            w.set_msc(true);
        });

        let msc = msc_regs();

        // Unlock MSC
        msc.lock().write(|w| w.set_lockkey(pac::msc::vals::Lockkey::UNLOCK));

        // Disable write enable (will be enabled per-operation)
        msc.writectrl().modify(|w| w.set_wren(false));

        Self { _private: () }
    }

    /// Get the flash base address.
    pub fn base_address(&self) -> u32 {
        FLASH_BASE
    }

    /// Get the total flash size in bytes.
    pub fn capacity(&self) -> u32 {
        FLASH_SIZE
    }

    /// Get the flash page size in bytes.
    pub fn page_size(&self) -> u32 {
        PAGE_SIZE
    }

    /// Read data from flash.
    ///
    /// This is a simple memory read since flash is memory-mapped.
    pub fn read(&self, address: u32, buffer: &mut [u8]) -> Result<(), Error> {
        if !self.is_valid_address(address, buffer.len() as u32) {
            return Err(Error::InvalidAddress);
        }

        // Flash is memory-mapped, so we can just copy directly
        let src = address as *const u8;
        unsafe {
            core::ptr::copy_nonoverlapping(src, buffer.as_mut_ptr(), buffer.len());
        }

        Ok(())
    }

    /// Erase flash pages.
    ///
    /// The address must be page-aligned and the size must be a multiple of the page size.
    #[allow(clippy::manual_is_multiple_of)]
    pub fn erase(&mut self, address: u32, size: u32) -> Result<(), Error> {
        // Check alignment
        if address % PAGE_SIZE != 0 || size % PAGE_SIZE != 0 {
            return Err(Error::Alignment);
        }

        // Check address range
        if !self.is_valid_address(address, size) {
            return Err(Error::InvalidAddress);
        }

        let msc = msc_regs();

        // Unlock and enable write
        let was_locked = self.is_locked();
        msc.lock().write(|w| w.set_lockkey(pac::msc::vals::Lockkey::UNLOCK));
        msc.writectrl().modify(|w| w.set_wren(true));

        let mut current_addr = address;
        let end_addr = address + size;

        while current_addr < end_addr {
            // Set address
            msc.addrb().write(|w| w.0 = current_addr);

            // Check for invalid address
            if msc.status().read().invaddr() {
                msc.writectrl().modify(|w| w.set_wren(false));
                if was_locked {
                    msc.lock().write(|w| w.set_lockkey(pac::msc::vals::Lockkey::LOCK));
                }
                return Err(Error::InvalidAddress);
            }

            // Issue erase page command
            msc.writecmd().write(|w| w.set_erasepage(true));

            // Wait for completion
            match self.wait_for_completion() {
                Ok(()) => {}
                Err(e) => {
                    msc.writectrl().modify(|w| w.set_wren(false));
                    if was_locked {
                        msc.lock().write(|w| w.set_lockkey(pac::msc::vals::Lockkey::LOCK));
                    }
                    return Err(e);
                }
            }

            current_addr += PAGE_SIZE;
        }

        // Disable write
        msc.writectrl().modify(|w| w.set_wren(false));

        if was_locked {
            msc.lock().write(|w| w.set_lockkey(pac::msc::vals::Lockkey::LOCK));
        }

        Ok(())
    }

    /// Write data to flash.
    ///
    /// The address must be word-aligned (4-byte) and the data length must be a multiple of 4.
    /// The flash should be erased before writing.
    #[allow(clippy::manual_is_multiple_of)]
    pub fn write(&mut self, address: u32, data: &[u8]) -> Result<(), Error> {
        // Check alignment
        if address % 4 != 0 || data.len() % 4 != 0 {
            return Err(Error::Alignment);
        }

        // Check address range
        if !self.is_valid_address(address, data.len() as u32) {
            return Err(Error::InvalidAddress);
        }

        if data.is_empty() {
            return Ok(());
        }

        let msc = msc_regs();

        // Unlock and enable write
        let was_locked = self.is_locked();
        msc.lock().write(|w| w.set_lockkey(pac::msc::vals::Lockkey::UNLOCK));
        msc.writectrl().modify(|w| w.set_wren(true));

        // Write data in bursts, respecting page boundaries
        let mut offset = 0u32;
        let mut current_addr = address;

        while offset < data.len() as u32 {
            // Calculate burst length (up to next page boundary)
            let page_remaining = PAGE_SIZE - (current_addr % PAGE_SIZE);
            let data_remaining = data.len() as u32 - offset;
            let burst_len = core::cmp::min(page_remaining, data_remaining);

            // Set address
            msc.addrb().write(|w| w.0 = current_addr);

            // Check for invalid address
            if msc.status().read().invaddr() {
                msc.writectrl().modify(|w| w.set_wren(false));
                if was_locked {
                    msc.lock().write(|w| w.set_lockkey(pac::msc::vals::Lockkey::LOCK));
                }
                return Err(Error::InvalidAddress);
            }

            // Write words
            let words = burst_len / 4;
            for i in 0..words {
                let word_offset = (offset + i * 4) as usize;
                let word = u32::from_le_bytes([
                    data[word_offset],
                    data[word_offset + 1],
                    data[word_offset + 2],
                    data[word_offset + 3],
                ]);

                // Write data word
                msc.wdata().write(|w| w.0 = word);

                // Wait for WDATAREADY (except for last word)
                if i < words - 1 {
                    match self.wait_for_wdata_ready() {
                        Ok(()) => {}
                        Err(e) => {
                            msc.writecmd().write(|w| w.set_writeend(true));
                            msc.writectrl().modify(|w| w.set_wren(false));
                            if was_locked {
                                msc.lock()
                                    .write(|w| w.set_lockkey(pac::msc::vals::Lockkey::LOCK));
                            }
                            return Err(e);
                        }
                    }
                }
            }

            // Issue write end command
            msc.writecmd().write(|w| w.set_writeend(true));

            // Wait for completion
            match self.wait_for_completion() {
                Ok(()) => {}
                Err(e) => {
                    msc.writectrl().modify(|w| w.set_wren(false));
                    if was_locked {
                        msc.lock().write(|w| w.set_lockkey(pac::msc::vals::Lockkey::LOCK));
                    }
                    return Err(e);
                }
            }

            offset += burst_len;
            current_addr += burst_len;
        }

        // Disable write
        msc.writectrl().modify(|w| w.set_wren(false));

        if was_locked {
            msc.lock().write(|w| w.set_lockkey(pac::msc::vals::Lockkey::LOCK));
        }

        Ok(())
    }

    /// Check if an address range is valid for flash operations.
    fn is_valid_address(&self, address: u32, len: u32) -> bool {
        (FLASH_BASE..FLASH_BASE + FLASH_SIZE).contains(&address)
            && address + len <= FLASH_BASE + FLASH_SIZE
    }

    /// Check if MSC registers are locked.
    fn is_locked(&self) -> bool {
        let msc = msc_regs();
        msc.status().read().reglock() == pac::msc::vals::Reglock::LOCKED
    }

    /// Wait for MSC operation to complete.
    fn wait_for_completion(&self) -> Result<(), Error> {
        let msc = msc_regs();
        let mut timeout = PROGRAM_TIMEOUT;

        while timeout > 0 {
            let status = msc.status().read();

            // Check for invalid address
            if status.invaddr() {
                return Err(Error::InvalidAddress);
            }

            // Check for locked flash
            if status.locked() || status.reglock() == pac::msc::vals::Reglock::LOCKED {
                return Err(Error::Locked);
            }

            // Check if operation is complete
            if !status.busy() && !status.pending() {
                return Ok(());
            }

            timeout -= 1;
        }

        Err(Error::Timeout)
    }

    /// Wait for WDATA register to be ready.
    fn wait_for_wdata_ready(&self) -> Result<(), Error> {
        let msc = msc_regs();
        let mut timeout = PROGRAM_TIMEOUT;

        while timeout > 0 {
            let status = msc.status().read();

            // Check for errors
            if status.invaddr() {
                return Err(Error::InvalidAddress);
            }
            if status.locked() || status.reglock() == pac::msc::vals::Reglock::LOCKED {
                return Err(Error::Locked);
            }

            // Check if ready for next word
            if status.wdataready() {
                return Ok(());
            }

            timeout -= 1;
        }

        Err(Error::Timeout)
    }
}

impl Default for Flash {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Flash {
    fn drop(&mut self) {
        let msc = msc_regs();

        // Disable write
        msc.writectrl().modify(|w| w.set_wren(false));

        // Lock MSC
        msc.lock().write(|w| w.set_lockkey(pac::msc::vals::Lockkey::LOCK));
    }
}

// ============================================================================
// Helper functions
// ============================================================================

#[inline]
fn msc_regs() -> pac::msc::Msc {
    #[cfg(feature = "_ns")]
    {
        pac::MSC_NS
    }
    #[cfg(not(feature = "_ns"))]
    {
        pac::MSC_S
    }
}

// ============================================================================
// embedded-storage trait implementations
// ============================================================================

/// Error type for embedded-storage traits.
impl embedded_storage::nor_flash::NorFlashError for Error {
    fn kind(&self) -> embedded_storage::nor_flash::NorFlashErrorKind {
        match self {
            Error::InvalidAddress => embedded_storage::nor_flash::NorFlashErrorKind::OutOfBounds,
            Error::Locked => embedded_storage::nor_flash::NorFlashErrorKind::Other,
            Error::Timeout => embedded_storage::nor_flash::NorFlashErrorKind::Other,
            Error::Alignment => embedded_storage::nor_flash::NorFlashErrorKind::NotAligned,
        }
    }
}

impl embedded_storage::nor_flash::ErrorType for Flash {
    type Error = Error;
}

impl embedded_storage::nor_flash::ReadNorFlash for Flash {
    const READ_SIZE: usize = 1;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        Flash::read(self, FLASH_BASE + offset, bytes)
    }

    fn capacity(&self) -> usize {
        FLASH_SIZE as usize
    }
}

impl embedded_storage::nor_flash::NorFlash for Flash {
    const WRITE_SIZE: usize = 4;
    const ERASE_SIZE: usize = PAGE_SIZE as usize;

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        Flash::erase(self, FLASH_BASE + from, to - from)
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        Flash::write(self, FLASH_BASE + offset, bytes)
    }
}

// ============================================================================
// Flash region utilities
// ============================================================================

/// Information about a flash region.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FlashRegion {
    /// Base address of the region.
    pub base: u32,
    /// Size of the region in bytes.
    pub size: u32,
}

impl Flash {
    /// Get information about the main flash region.
    pub fn main_flash_region(&self) -> FlashRegion {
        FlashRegion {
            base: FLASH_BASE,
            size: FLASH_SIZE,
        }
    }

    /// Verify that data was written correctly.
    pub fn verify(&self, address: u32, data: &[u8]) -> Result<bool, Error> {
        if !self.is_valid_address(address, data.len() as u32) {
            return Err(Error::InvalidAddress);
        }

        let flash_ptr = address as *const u8;
        for (i, &byte) in data.iter().enumerate() {
            let flash_byte = unsafe { core::ptr::read_volatile(flash_ptr.add(i)) };
            if flash_byte != byte {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check if a flash region is erased (all 0xFF).
    pub fn is_erased(&self, address: u32, size: u32) -> Result<bool, Error> {
        if !self.is_valid_address(address, size) {
            return Err(Error::InvalidAddress);
        }

        let flash_ptr = address as *const u32;
        let words = size / 4;

        for i in 0..words {
            let word = unsafe { core::ptr::read_volatile(flash_ptr.add(i as usize)) };
            if word != 0xFFFF_FFFF {
                return Ok(false);
            }
        }

        // Check remaining bytes
        let remaining = size % 4;
        if remaining > 0 {
            let byte_ptr = (address + words * 4) as *const u8;
            for i in 0..remaining {
                let byte = unsafe { core::ptr::read_volatile(byte_ptr.add(i as usize)) };
                if byte != 0xFF {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }
}
