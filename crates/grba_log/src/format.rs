use capstone::prelude::*;
use owo_colors::OwoColorize;

use grba_core::emulator::cpu::registers::{Mode, State, PSR};
use tabled::{builder, Column, Concat, Format, Modify, Style, Tabled};
use zerocopy::{ByteSlice, LayoutVerified};

/// The format from the Logs that we have from other emulators.
/// Should really use `U32<LittleEndian>`, but that doesn't implement hex debug print :(
#[derive(zerocopy::FromBytes, Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Default)]
#[repr(C)]
pub struct InstructionSnapshot {
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
    r4: u32,
    r5: u32,
    r6: u32,
    r7: u32,
    r8: u32,
    r9: u32,
    r10: u32,
    r11: u32,
    r12: u32,
    r13: u32,
    r14: u32,
    r15: u32,
    cpsr: u32,
    spsr: u32,
}

impl InstructionSnapshot {
    pub fn parse<B: ByteSlice>(bytes: B) -> Option<LayoutVerified<B, [Self]>> {
        let result = zerocopy::LayoutVerified::new_slice(bytes)?;

        Some(result)
    }

    pub fn get_differing_fields(&self, other: &Self) -> Vec<usize> {
        let mut differing_fields = Vec::new();

        if self.r0 != other.r0 {
            differing_fields.push(0);
        }
        if self.r1 != other.r1 {
            differing_fields.push(1);
        }
        if self.r2 != other.r2 {
            differing_fields.push(2);
        }
        if self.r3 != other.r3 {
            differing_fields.push(3);
        }
        if self.r4 != other.r4 {
            differing_fields.push(4);
        }
        if self.r5 != other.r5 {
            differing_fields.push(5);
        }
        if self.r6 != other.r6 {
            differing_fields.push(6);
        }
        if self.r7 != other.r7 {
            differing_fields.push(7);
        }
        if self.r8 != other.r8 {
            differing_fields.push(8);
        }
        if self.r9 != other.r9 {
            differing_fields.push(9);
        }
        if self.r10 != other.r10 {
            differing_fields.push(10);
        }
        if self.r11 != other.r11 {
            differing_fields.push(11);
        }
        if self.r12 != other.r12 {
            differing_fields.push(12);
        }
        if self.r13 != other.r13 {
            differing_fields.push(13);
        }
        if self.r14 != other.r14 {
            differing_fields.push(14);
        }
        if self.r15 != other.r15 {
            differing_fields.push(15);
        }
        if self.cpsr != other.cpsr {
            differing_fields.push(16);
        }
        if self.spsr != other.spsr {
            differing_fields.push(17);
        }

        differing_fields
    }
}

impl From<grba_core::logging::InstructionSnapshot> for InstructionSnapshot {
    fn from(snap: grba_core::logging::InstructionSnapshot) -> Self {
        // Since we're using the C layout, we can just cast the pointer
        unsafe { std::mem::transmute(snap) }
    }
}

impl AsRef<InstructionSnapshot> for grba_core::logging::InstructionSnapshot {
    fn as_ref(&self) -> &InstructionSnapshot {
        // Since we're using the C layout, we can just cast the pointer
        unsafe { std::mem::transmute(self) }
    }
}

impl Tabled for InstructionSnapshot {
    const LENGTH: usize = 18;
    fn fields(&self) -> Vec<String> {
        let mut out = Vec::with_capacity(Self::LENGTH);
        out.push(format!("{:#010X}", self.r0));
        out.push(format!("{:#010X}", self.r1));
        out.push(format!("{:#010X}", self.r2));
        out.push(format!("{:#010X}", self.r3));
        out.push(format!("{:#010X}", self.r4));
        out.push(format!("{:#010X}", self.r5));
        out.push(format!("{:#010X}", self.r6));
        out.push(format!("{:#010X}", self.r7));
        out.push(format!("{:#010X}", self.r8));
        out.push(format!("{:#010X}", self.r9));
        out.push(format!("{:#010X}", self.r10));
        out.push(format!("{:#010X}", self.r11));
        out.push(format!("{:#010X}", self.r12));
        out.push(format!("{:#010X}", self.r13));
        out.push(format!("{:#010X}", self.r14));
        out.push(format!("{:#010X}", self.r15));
        out.push(format!("{:#010X}", self.cpsr));
        out.push(format!("{:#010X}", self.spsr));
        out
    }

    fn headers() -> Vec<String> {
        let mut out = Vec::with_capacity(Self::LENGTH);

        out.push(String::from("r0"));
        out.push(String::from("r1"));
        out.push(String::from("r2"));
        out.push(String::from("r3"));
        out.push(String::from("r4"));
        out.push(String::from("r5"));
        out.push(String::from("r6"));
        out.push(String::from("r7"));
        out.push(String::from("r8"));
        out.push(String::from("r9"));
        out.push(String::from("r10"));
        out.push(String::from("r11"));
        out.push(String::from("r12"));
        out.push(String::from("r13"));
        out.push(String::from("r14"));
        out.push(String::from("r15"));
        out.push(String::from("cpsr"));
        out.push(String::from("spsr"));

        out
    }
}

#[derive(Debug)]
pub struct DiffItem<'a> {
    /// The index of the executed instruction in the log
    pub instr_idx: usize,
    /// Whether this was the first difference causing an error
    pub is_error: bool,
    /// The indexes of the fields from [InstructionSnapshot]s which are different
    pub different_fields: Vec<usize>,
    /// The [InstructionSnapshot] from the emulator log
    pub emu_instr: &'a InstructionSnapshot,
    /// The [InstructionSnapshot] from the other emulator log
    pub other_instr: &'a InstructionSnapshot,
}

impl<'a> Tabled for DiffItem<'a> {
    const LENGTH: usize = 2;

    fn fields(&self) -> Vec<String> {
        {
            let mut out = Vec::with_capacity(Self::LENGTH);

            if self.is_error {
                out.push(format!(
                    "{}\n{}",
                    self.instr_idx.bright_magenta(),
                    "(X)".bright_magenta()
                ));
            } else {
                out.push(format!("{}", self.instr_idx));
            }

            let name_table = builder::Builder::new()
                .set_header(["Emulator"])
                .add_row(["Emu"])
                .add_row(["Other"])
                .build();

            let mut register_table = tabled::Table::new([self.emu_instr, self.other_instr]);

            for &column_idx in &self.different_fields {
                register_table = register_table
                    .with(Modify::new(Column(column_idx..=column_idx)).with(Format(|s| s.bright_red().to_string())));
            }

            let table = name_table
                .with(Concat::horizontal(register_table))
                .with(Style::PSEUDO_CLEAN);

            out.push(format!("{}", table));

            out
        }
    }

    fn headers() -> Vec<String> {
        vec!["Index".to_string(), "Registers".to_string()]
    }
}

#[derive(Debug)]
pub struct DiffItemWithInstr<'a> {
    pub diff_item: DiffItem<'a>,
    pub instr: u32,
}

impl<'a> tabled::Tabled for DiffItemWithInstr<'a> {
    const LENGTH: usize = 1 + DiffItem::LENGTH;

    fn fields(&self) -> Vec<String> {
        {
            let mut out = Vec::with_capacity(Self::LENGTH);
            out.extend(self.diff_item.fields());

            let cpsr = PSR::from_raw(self.diff_item.emu_instr.cpsr);

            let current_mode = match cpsr.state() {
                State::Arm => capstone::arch::arm::ArchMode::Arm,
                State::Thumb => capstone::arch::arm::ArchMode::Thumb,
            };

            let capstone = capstone::Capstone::new()
                .arm()
                .mode(current_mode)
                .syntax(arch::arm::ArchSyntax::NoRegName)
                .detail(true)
                .build()
                .unwrap();

            let disassembled = capstone
                .disasm_all(&self.instr.to_le_bytes(), self.diff_item.emu_instr.r15 as u64)
                .unwrap();

            if let Some(instr) = disassembled.get(0) {
                out.push(format!(
                    "{} {}\n{:?}\n{:#X}",
                    instr.mnemonic().unwrap(),
                    instr.op_str().unwrap(),
                    current_mode,
                    self.instr
                ));
            } else {
                out.push(format!("ERROR\n{:?}\n{:#X}", current_mode, self.instr));
            }

            out
        }
    }

    fn headers() -> Vec<String> {
        let mut out = Vec::with_capacity(Self::LENGTH);
        out.extend(DiffItem::headers());
        out.push(String::from("Disassembly"));
        out
    }
}
