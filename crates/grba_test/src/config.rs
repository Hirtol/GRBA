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
    /// The amount of frames to emulate initially
    pub num_frames: u32,
    /// The sequence of instructions to run *after* the initial `num_frames`
    #[serde(default)]
    pub sequences: HashMap<String, Vec<TestSequenceInstructions>>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum TestSequenceInstructions {
    /// Dump the current frame and use it for future comparisons in the test-runner.
    /// The dump's ID will be the `rom_id + DumpName`.
    DumpFrame(String),
    /// Advance the given number of frames
    AdvanceFrames(u32),
    /// Provide the given input for the next frame.
    ///
    /// Will run 2 frames implicitly, one with the key pressed, and then one where it is released.
    Input(grba_core::InputKeys),
    /// Hold the given input for the given amount of frames.
    ///
    /// Will implicitly run `n + 1` frames as the input has to be released for at least one frame
    HoldInputFor(grba_core::InputKeys, u32),
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

pub fn save_config(config: &TestConfig) -> anyhow::Result<()> {
    let path = Path::new("./grba_test_conf.json");

    serde_json::to_writer_pretty(std::fs::File::create(path)?, config)?;

    Ok(())
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
