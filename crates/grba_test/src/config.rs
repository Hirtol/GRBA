use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

pub type RomName = String;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TestConfig {
    pub num_threads: NonZeroUsize,
    /// The path of the directory with all test roms
    pub test_rom_dir: PathBuf,
    pub output_path: PathBuf,
    pub snapshot_path: PathBuf,
    pub bios_path: PathBuf,
    pub custom_configs: HashMap<RomName, CustomRomTest>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CustomRomTest {
    /// The amount of frames to emulate
    pub num_frames: u32,
}

pub enum TestSequenceInstructions {
    DumpFrame,
    AdvanceFrames(u32),
    Input(grba_core::InputKeys),
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            num_threads: std::thread::available_parallelism().unwrap(),
            test_rom_dir: PathBuf::from("./test_roms"),
            output_path: PathBuf::from("./grba_test_output"),
            snapshot_path: PathBuf::from("./test_roms/expected"),
            bios_path: PathBuf::from("./roms/gba_bios.bin"),
            custom_configs: Default::default(),
        }
    }
}

pub fn load_config() -> anyhow::Result<TestConfig> {
    let path = Path::new("./grba_test_conf.json");

    if path.exists() {
        Ok(serde_json::from_reader(std::fs::File::open(path)?)?)
    } else {
        let defaults = TestConfig::default();
        println!("No test config exists, creating default at: `{:#?}`", path);

        serde_json::to_writer_pretty(std::fs::File::create(path)?, &defaults)?;

        Ok(defaults)
    }
}

#[derive(clap::Parser, Debug)]
#[clap(version, about)]
pub struct ClapArgs {
    /// The path of the directory with all test roms, if not provided the config's value will be used
    pub test_rom_dir: Option<PathBuf>,
    /// The path where the results will be dumped.
    #[clap(short)]
    pub output_path: Option<PathBuf>,
    #[clap(short)]
    pub bios: Option<PathBuf>,
    /// The amount of frames to emulate
    #[clap(short, default_value = "5")]
    pub frames: u32,
    /// The amount of threads to use, by default will use as many threads as the system has.
    pub num_threads: Option<NonZeroUsize>,
}
