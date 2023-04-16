use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::path::Path;

use emu_test_runner::inputs::{get_rom_fs_id, TestCandidate};

use crate::config::{TestConfig, TestSequenceInstructions};

pub struct CustomRomTestSequence<'a> {
    pub num_frames: u32,
    pub sequence: Option<&'a Vec<TestSequenceInstructions>>,
}

pub fn find_all_tests<'a>(
    path: &Path,
    config: &'a TestConfig,
) -> anyhow::Result<(Vec<TestCandidate>, HashMap<String, CustomRomTestSequence<'a>>)> {
    let files = emu_test_runner::inputs::list_files_with_extensions(path, ".gba")?;
    let mut sequences = HashMap::new();

    let out = files
        .into_iter()
        .flat_map(|path| {
            let basic_rom_id = get_rom_fs_id(&path);

            if let Some(cfg) = config.custom_configs.get(basic_rom_id.as_ref()) {
                if !cfg.sequences.is_empty() {
                    return cfg
                        .sequences
                        .iter()
                        .inspect(|(test_name, sequence)| {
                            sequences.insert(
                                format!("{basic_rom_id}_{test_name}"),
                                CustomRomTestSequence {
                                    num_frames: cfg.num_frames,
                                    sequence: Some(sequence),
                                },
                            );
                        })
                        .map(|(test_name, _)| TestCandidate::new(format!("{basic_rom_id}_{test_name}"), path.clone()))
                        .collect();
                } else {
                    sequences.insert(
                        basic_rom_id.to_string(),
                        CustomRomTestSequence {
                            num_frames: cfg.num_frames,
                            sequence: None,
                        },
                    );
                }
            }

            vec![TestCandidate::new(basic_rom_id.into_owned(), path)]
        })
        .collect();

    Ok((out, sequences))
}

pub struct MemoryRam {
    data: Box<[u8; grba_core::emulator::cartridge::CARTRIDGE_RAM_SIZE]>,
}

impl Default for MemoryRam {
    fn default() -> Self {
        Self {
            data: grba_core::box_array![0; grba_core::emulator::cartridge::CARTRIDGE_RAM_SIZE],
        }
    }
}

impl Deref for MemoryRam {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &*self.data
    }
}

impl DerefMut for MemoryRam {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.data
    }
}
