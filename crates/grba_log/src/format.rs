use std::fmt::{Display, Formatter};
use tabled::Tabled;
use zerocopy::{ByteSlice, LayoutVerified};

/// The format from the Logs that we have from other emulators.
/// Should really use `U32<LittleEndian>`, but that doesn't implement hex debug print :(
#[derive(zerocopy::FromBytes, Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
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

impl Display for InstructionSnapshot {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let tb = tabled::Table::new([self]);
        write!(f, "{}", tb)
    }
}

impl Tabled for InstructionSnapshot {
    const LENGTH: usize = 18;
    fn fields(&self) -> Vec<String> {
        let mut out = Vec::new();
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
        let mut out = Vec::new();

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
