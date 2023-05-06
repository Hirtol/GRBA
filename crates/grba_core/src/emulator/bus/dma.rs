use modular_bitfield::prelude::B5;
use modular_bitfield::{bitfield, BitfieldSpecifier};

use crate::emulator::bus::interrupts::{InterruptManager, Interrupts};
use crate::emulator::bus::Bus;
use crate::emulator::cpu::CPU;
use crate::emulator::{AlignedAddress, MemoryAddress};
use crate::scheduler::{EmuTime, EventTag, Scheduler};
use crate::utils::BitOps;

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

const DMA_SRC_ADDRESS_MASKS: [u32; 4] = [0x07FFFFFF, 0x0FFFFFFF, 0x0FFFFFFF, 0x0FFFFFFF];
const DMA_DST_ADDRESS_MASKS: [u32; 4] = [0x07FFFFFF, 0x07FFFFFF, 0x07FFFFFF, 0x0FFFFFFF];

const WORD_COUNT_MASK: [u32; 4] = [0x3FFF, 0x3FFF, 0x3FFF, 0xFFFF];

// I really hate doing this, but DMA does require BUS access and code locality is more valuable here.
impl Bus {
    pub fn on_dma_start(&mut self, cpu: &CPU, channel_idx: usize) {
        // 2 Set non-sequential read cycles for every DMA.
        self.scheduler.add_time(2);
        let mut channel = self.dma.channels[channel_idx];
        let mut transfer_state = &mut channel.current_transfer;

        // TODO: Make this not an instant transfer by ticking scheduler & checking for higher priority DMAs
        match channel.control.dma_transfer_type() {
            DmaTransferType::U16 => {
                for _ in 0..transfer_state.length {
                    let value = self.read_16(transfer_state.source_address, cpu);
                    self.write_16(transfer_state.dest_address, value);
                    // Two's complement allows us to just cast i32 to u32 for this
                    transfer_state.dest_address += channel.control.dest_addr_control().to_address_offset_u16() as u32;
                    transfer_state.source_address += channel.control.src_addr_control().to_address_offset_u16() as u32;
                }
            }
            DmaTransferType::U32 => {
                for _ in 0..transfer_state.length {
                    let value = self.read_32(transfer_state.source_address, cpu);
                    self.write_32(transfer_state.dest_address, value);
                    // Two's complement allows us to just cast i32 to u32 for this
                    transfer_state.dest_address += channel.control.dest_addr_control().to_address_offset_u32() as u32;
                    transfer_state.source_address += channel.control.src_addr_control().to_address_offset_u32() as u32;
                }
            }
        }

        // Interrupt requests
        if channel.control.irq_on_end_of_word_count() {
            let interrupt = match channel_idx {
                0 => Interrupts::DMA0,
                1 => Interrupts::DMA1,
                2 => Interrupts::DMA2,
                3 => Interrupts::DMA3,
                _ => unreachable!(),
            };

            self.interrupts.request_interrupt(interrupt, &mut self.scheduler);
        }

        // Repeat
        if channel.control.dma_repeat() && channel.control.dma_start_timing() != DmaStartTiming::Immediately {
            if channel.control.dest_addr_control() == DmaAddrControl::IncrReload {
                channel.current_transfer.dest_address = channel.masked_dest(channel_idx);
            }

            channel.current_transfer.source_address = channel.masked_source(channel_idx);
            channel.current_transfer.length = channel.masked_word_count(channel_idx);
        } else {
            channel.control.set_dma_enable(false);
        }
    }

    /// At the moment we'll just poll.
    ///
    /// This can be implemented more efficiently by keeping 2 sorted Vecs (HBLANK,VBLANK) with current channels.
    pub fn poll_dmas(&mut self, cpu: &CPU, start_time: DmaStartTiming) {
        for i in 0..4 {
            let channel = &self.dma.channels[i];
            if channel.control.dma_start_timing() == start_time && channel.control.dma_enable() {
                self.on_dma_start(cpu, i)
            }
        }
    }
}

pub struct DmaChannels {
    /// DMA0 - highest priority, best for timing critical transfers (eg. HBlank DMA).
    /// DMA1 and DMA2 - can be used to feed digital sample data to the Sound FIFOs.
    /// DMA3 - can be used to write to Game Pak ROM/FlashROM (but not GamePak SRAM).
    /// Beside for that, each DMA 0-3 may be used for whatever general purposes.
    channels: [DmaChannel; 4],
    /// Whichever DMA is currently active.
    ///
    /// Higher priority DMAs can interrupt lower priority ones, so we need to know if we're currently running one.
    current_dma: Option<usize>,
}

impl DmaChannels {
    pub fn new() -> Self {
        Self {
            channels: [DmaChannel::new(); 4],
            current_dma: None,
        }
    }

    pub fn channel(&self, channel: usize) -> &DmaChannel {
        &self.channels[channel]
    }

    pub fn channel_mut(&mut self, channel: usize) -> &mut DmaChannel {
        &mut self.channels[channel]
    }

    #[inline]
    pub fn write_channel(&mut self, address: AlignedAddress, value: u8, scheduler: &mut Scheduler) {
        match address {
            DMA_0_ADDR_START..=DMA_0_ADDR_END => {
                self.channels[0].write((address - DMA_0_ADDR_START) as usize, value, scheduler, 0)
            }
            DMA_1_ADDR_START..=DMA_1_ADDR_END => {
                self.channels[1].write((address - DMA_1_ADDR_START) as usize, value, scheduler, 1)
            }
            DMA_2_ADDR_START..=DMA_2_ADDR_END => {
                self.channels[2].write((address - DMA_2_ADDR_START) as usize, value, scheduler, 2)
            }
            DMA_3_ADDR_START..=DMA_3_ADDR_END => {
                self.channels[3].write((address - DMA_3_ADDR_START) as usize, value, scheduler, 3)
            }
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
    current_transfer: DmaTransferState,
}

#[derive(Debug, Clone, Copy)]
struct DmaTransferState {
    pub source_address: MemoryAddress,
    pub dest_address: MemoryAddress,
    pub length: u32,
}

impl DmaChannel {
    pub fn new() -> Self {
        Self {
            source_address: 0,
            dest_address: 0,
            word_count: 0,
            control: DmaControl::new(),
            current_transfer: DmaTransferState {
                source_address: 0,
                dest_address: 0,
                length: 0,
            },
        }
    }

    #[inline]
    pub fn write(&mut self, offset: usize, value: u8, scheduler: &mut Scheduler, channel_idx: usize) {
        match offset {
            0..=3 => self.source_address.set_byte_le(offset, value),
            DMA_DEST_ADDR_OFFSET..=7 => self.dest_address.set_byte_le(offset - DMA_DEST_ADDR_OFFSET, value),
            DMA_WORD_CNT_OFFSET..=9 => self.word_count.set_byte_le(offset - DMA_WORD_CNT_OFFSET, value),
            DMA_CONTROL_OFFSET..=11 => {
                let old = self.control;

                self.control.update_byte_le(offset - DMA_CONTROL_OFFSET, value);

                if self.control.dma_enable() && !old.dma_enable() {
                    crate::cpu_log!("bus-logging"; "Enabling DMA: `{}` at clock cycle: `{:?}` with state {:#?}", channel_idx, scheduler.current_time, self);

                    self.current_transfer = DmaTransferState {
                        source_address: self.masked_source(channel_idx),
                        dest_address: self.masked_dest(channel_idx),
                        length: self.masked_word_count(channel_idx),
                    };

                    match self.control.dma_start_timing() {
                        // TODO: 2 cycle activation delay when we have proper timing
                        DmaStartTiming::Immediately => {
                            scheduler.schedule_relative(EventTag::DmaStart(channel_idx), EmuTime(0))
                        }
                        DmaStartTiming::Special => {
                            // TODO: Sound FIFO DMA1/DMA2
                        }
                        _ => {}
                    }
                }
            }
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

    #[inline(always)]
    fn masked_source(&self, channel_idx: usize) -> u32 {
        self.source_address & DMA_SRC_ADDRESS_MASKS[channel_idx]
    }

    #[inline(always)]
    fn masked_dest(&self, channel_idx: usize) -> u32 {
        self.dest_address & DMA_DST_ADDRESS_MASKS[channel_idx]
    }

    #[inline(always)]
    fn masked_word_count(&self, channel_idx: usize) -> u32 {
        match self.word_count as u32 & WORD_COUNT_MASK[channel_idx] {
            0 => WORD_COUNT_MASK[channel_idx] + 1,
            len => len,
        }
    }
}

#[bitfield(bits = 16)]
#[repr(u16)]
#[derive(Debug, Copy, Clone)]
pub struct DmaControl {
    #[skip]
    unused: B5,
    pub dest_addr_control: DmaAddrControl,
    /// `IncrReload` is technically forbidden in `src_addr`
    pub src_addr_control: DmaAddrControl,
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
pub enum DmaAddrControl {
    Increment = 0b00,
    Decrement = 0b01,
    Fixed = 0b10,
    IncrReload = 0b11,
}

impl DmaAddrControl {
    #[inline(always)]
    pub fn to_address_offset_u32(self) -> i32 {
        const ADDRESS_CONTROL_OFFSETS: [i32; 4] = [4, -4, 0, 4];
        ADDRESS_CONTROL_OFFSETS[self as usize]
    }

    #[inline(always)]
    pub fn to_address_offset_u16(self) -> i32 {
        const ADDRESS_CONTROL_OFFSETS: [i32; 4] = [2, -2, 0, 2];
        ADDRESS_CONTROL_OFFSETS[self as usize]
    }
}

#[derive(Debug, BitfieldSpecifier, PartialEq, Clone, Copy)]
#[bits = 1]
pub enum DmaTransferType {
    U16 = 0b0,
    U32 = 0b1,
}

#[derive(Debug, BitfieldSpecifier, PartialEq, Clone, Copy)]
#[bits = 2]
pub enum DmaStartTiming {
    Immediately = 0b00,
    VBlank = 0b01,
    HBlank = 0b10,
    Special = 0b11,
}
