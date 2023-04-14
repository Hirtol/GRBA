use std::fs::File;
use std::path::PathBuf;

use platform_dirs::AppDirs;

use crate::gui::AppUiState;
use crate::EguiFramework;

pub const CONFIG_FILE: &str = "config.toml";
pub const GUI_STATE_FILE: &str = "gui_state.bin";

pub fn save_state_and_config(gui: &EguiFramework) -> anyhow::Result<()> {
    let persistence = get_persistences_dir();

    std::fs::create_dir_all(&persistence)?;

    let mut file = File::create(persistence.join(GUI_STATE_FILE))?;

    let ui_state = AppUiState {
        debug_ui: gui.gui.debug_view.state.clone(),
        egui: gui.memory(),
    };

    bincode::serialize_into(file, &ui_state)?;

    Ok(())
}

pub fn deserialise_state_and_config() -> Option<AppUiState> {
    let mut file = File::open(get_persistences_dir().join(GUI_STATE_FILE)).ok()?;

    bincode::deserialize_from(file).ok()
}

pub fn get_full_config_path() -> PathBuf {
    get_app_dirs().config_dir.join(CONFIG_FILE)
}

pub fn get_save_states_dir() -> PathBuf {
    get_app_dirs().data_dir.join("save_states")
}

pub fn get_persistences_dir() -> PathBuf {
    get_app_dirs().data_dir
}

pub fn get_app_dirs() -> AppDirs {
    platform_dirs::AppDirs::new("GRBA".into(), false).expect("Couldn't find a home directory for config!")
}
