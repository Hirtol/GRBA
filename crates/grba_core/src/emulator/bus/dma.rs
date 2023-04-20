use crate::emulator::{AlignedAddress, MemoryAddress};
use crate::utils::BitOps;
use modular_bitfield::prelude::B5;
use modular_bitfield::{bitfield, BitfieldSpecifier};

pub const DMA_CHANNEL_SIZE: usize = 12;
pub const DMA_DEST_ADDR_OFFSET: usize = 4;
pub const DMA_WORD_CNT_OFFSET: usize = 8;
pub const DMA_CONTROL_OFFSET: usize = 10;

pub const DMA_0_ADDR_START: MemoryAddress = 0x0400_00B0;
pub const DMA_0_ADDR_END: MemoryAddress = 0x0400_00BB;
pub const DMA_1_ADDR_START: MemoryAddress = 0x0400_00BC;
pub const DMA_1_ADDR_END: MemoryAddress = 0x0400_00C7;
pub const DMA_2_ADDR_START: MemoryAddress = 0x0400_00C8;
pub const DMA_2_ADDR_END: MemoryAddress = 0x0400_00D3;
pub const DMA_3_ADDR_START: MemoryAddress = 0x0400_00D4;
pub const DMA_3_ADDR_END: MemoryAddress = 0x0400_00DF;

pub const DMA_0_CONTROL_START: MemoryAddress = 0x0400_00BA;
pub const DMA_0_CONTROL_END: MemoryAddress = DMA_0_ADDR_END;
pub const DMA_1_CONTROL_START: MemoryAddress = 0x0400_00C6;
pub const DMA_1_CONTROL_END: MemoryAddress = DMA_1_ADDR_END;
pub const DMA_2_CONTROL_START: MemoryAddress = 0x0400_00D2;
pub const DMA_2_CONTROL_END: MemoryAddress = DMA_2_ADDR_END;
pub const DMA_3_CONTROL_START: MemoryAddress = 0x0400_00DE;
pub const DMA_3_CONTROL_END: MemoryAddress = DMA_3_ADDR_END;

pub struct DmaChannels {
    /// DMA0 - highest priority, best for timing critical transfers (eg. HBlank DMA).
    /// DMA1 and DMA2 - can be used to feed digital sample data to the Sound FIFOs.
    /// DMA3 - can be used to write to Game Pak ROM/FlashROM (but not GamePak SRAM).
    /// Beside for that, each DMA 0-3 may be used for whatever general purposes.
    channels: [DmaChannel; 4],
}

impl DmaChannels {
    pub fn new() -> Self {
        Self {
            channels: [DmaChannel::new(); 4],
        }
    }

    pub fn channel(&self, channel: usize) -> &DmaChannel {
        &self.channels[channel]
    }

    pub fn channel_mut(&mut self, channel: usize) -> &mut DmaChannel {
        &mut self.channels[channel]
    }

    #[inline]
    pub fn write_channel(&mut self, address: AlignedAddress, value: u8) {
        match address {
            DMA_0_ADDR_START..=DMA_0_ADDR_END => self.channels[0].write((address - DMA_0_ADDR_START) as usize, value),
            DMA_1_ADDR_START..=DMA_1_ADDR_END => self.channels[1].write((address - DMA_1_ADDR_START) as usize, value),
            DMA_2_ADDR_START..=DMA_2_ADDR_END => self.channels[2].write((address - DMA_2_ADDR_START) as usize, value),
            DMA_3_ADDR_START..=DMA_3_ADDR_END => self.channels[3].write((address - DMA_3_ADDR_START) as usize, value),
            _ => unreachable!(),
        }
    }

    /// Read the register values ignoring write-only properties
    pub fn read_debug(&self, address: AlignedAddress) -> u8 {
        match address {
            DMA_0_ADDR_START..=DMA_0_ADDR_END => self.channels[0].read_debug((address - DMA_0_ADDR_START) as usize),
            DMA_1_ADDR_START..=DMA_1_ADDR_END => self.channels[1].read_debug((address - DMA_1_ADDR_START) as usize),
            DMA_2_ADDR_START..=DMA_2_ADDR_END => self.channels[2].read_debug((address - DMA_2_ADDR_START) as usize),
            DMA_3_ADDR_START..=DMA_3_ADDR_END => self.channels[3].read_debug((address - DMA_3_ADDR_START) as usize),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DmaChannel {
    source_address: MemoryAddress,
    dest_address: MemoryAddress,
    word_count: u16,
    control: DmaControl,
}

impl DmaChannel {
    pub fn new() -> Self {
        Self {
            source_address: 0,
            dest_address: 0,
            word_count: 0,
            control: DmaControl::new(),
        }
    }

    #[inline]
    pub fn write(&mut self, offset: usize, value: u8) {
        match offset {
            0..=3 => self.source_address.set_byte_le(offset, value),
            DMA_DEST_ADDR_OFFSET..=7 => self.dest_address.set_byte_le(offset - DMA_DEST_ADDR_OFFSET, value),
            DMA_WORD_CNT_OFFSET..=9 => self.word_count.set_byte_le(offset - DMA_WORD_CNT_OFFSET, value),
            DMA_CONTROL_OFFSET..=11 => self.control.update_byte_le(offset - DMA_CONTROL_OFFSET, value),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn control(&self) -> DmaControl {
        self.control
    }

    pub fn read_debug(&self, offset: usize) -> u8 {
        match offset {
            0..=3 => self.source_address.to_le_bytes()[offset],
            DMA_DEST_ADDR_OFFSET..=7 => self.dest_address.to_le_bytes()[offset - DMA_DEST_ADDR_OFFSET],
            DMA_WORD_CNT_OFFSET..=9 => self.word_count.to_le_bytes()[offset - DMA_WORD_CNT_OFFSET],
            DMA_CONTROL_OFFSET..=11 => self.control.to_le_bytes()[offset - DMA_CONTROL_OFFSET],
            _ => unreachable!(),
        }
    }
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[derive(Debug, Copy, Clone)]
pub struct DmaControl {
    #[skip]
    unused: B5,
    pub dest_addr_control: DmaAddrControlDest,
    pub src_addr_control: DmaAddrControlSrc,
    /// (Must be zero if Bit 11 set)
    pub dma_repeat: bool,
    pub dma_transfer_type: DmaTransferType,
    /// DMA3 only
    /// 0 = Normal, 1 = DRQ <from> Game Pak, DMA3
    pub game_pak_drq: bool,
    /// The 'Special' setting (Start Timing=3) depends on the DMA channel:
    /// DMA0=Prohibited, DMA1/DMA2=Sound FIFO, DMA3=Video Capture
    pub dma_start_timing: DmaStartTiming,
    pub irq_on_end_of_word_count: bool,
    /// After enabling the DMA is delayed by 2 cycles. (Technically, probably won't implement that :) ).
    /// TODO: Implement when we have accurate timings.
    pub dma_enable: bool,
}

#[derive(Debug, BitfieldSpecifier, PartialEq, Clone, Copy)]
#[bits = 2]
pub enum DmaAddrControlDest {
    Increment = 0b00,
    Decrement = 0b01,
    Fixed = 0b10,
    IncrReload = 0b11,
}

#[derive(Debug, BitfieldSpecifier, PartialEq, Clone, Copy)]
#[bits = 2]
pub enum DmaAddrControlSrc {
    Increment = 0b00,
    Decrement = 0b01,
    Fixed = 0b10,
    Prohibited = 0b11,
}

#[derive(Debug, BitfieldSpecifier, PartialEq, Clone, Copy)]
#[bits = 1]
pub enum DmaTransferType {
    Bit16 = 0b0,
    Bit32 = 0b1,
}

#[derive(Debug, BitfieldSpecifier, PartialEq, Clone, Copy)]
#[bits = 2]
pub enum DmaStartTiming {
    Immediately = 0b00,
    VBlank = 0b01,
    HBlank = 0b10,
    Special = 0b11,
}
