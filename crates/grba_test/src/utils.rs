use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

/// Lists all files in the provided `path` (if the former is a directory) with the provided
/// `extension`
pub fn list_files_with_extensions(path: impl AsRef<Path>, extension: impl AsRef<str>) -> anyhow::Result<Vec<PathBuf>> {
    let mut result = Vec::with_capacity(200);

    if path.as_ref().is_dir() {
        for entry in std::fs::read_dir(path)? {
            let path = entry?.path();
            if path.is_dir() {
                result.extend(list_files_with_extensions(&path, extension.as_ref())?);
            } else if path.to_str().filter(|t| t.ends_with(extension.as_ref())).is_some() {
                result.push(path);
            }
        }
    }

    Ok(result)
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
