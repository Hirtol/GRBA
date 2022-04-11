use std::ops::RangeInclusive;

use egui::{Slider, Ui, Widget};
use enum_iterator::IntoEnumIterator;

use grba_core::emulator::debug::{BgMode, LCD_CONTROL_END, LCD_CONTROL_START, LCD_STATUS_END, LCD_STATUS_START};
use grba_core::emulator::MemoryAddress;

pub trait ViewableRegister {
    /// The name of the register.
    ///
    /// Used to display the register name in the UI.
    fn get_name(&self) -> &'static str;

    /// An optional description of the register.
    fn get_description(&self) -> &'static str {
        ""
    }

    /// Total amount of registers associated with this register concept.
    ///
    /// An example could be `BgControl`, of which there are 4 instances
    fn get_total_instances(&self) -> u32 {
        1
    }

    /// Display the current register value as a hex coded `u8`/`u16`/`u32`.
    fn get_current_value(&self, reg_value: &[u8]) -> String;

    /// Get the address of the register
    fn get_address(&self, instance: u32) -> RangeInclusive<MemoryAddress>;

    /// Draw the register content within the provided `ui`.
    ///
    /// The `reg_value` is the value of the memory located at [Self::get_address].
    ///
    /// # Returns
    ///
    /// [Some] if the register was changed by the user, [None] otherwise.
    fn draw(&self, ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>>;
}

pub fn get_register_list() -> Vec<&'static dyn ViewableRegister> {
    vec![&DispCntView, &DispStatView]
}

macro_rules! checkbox {
    ($ui:expr, $reg:expr, $set_method:ident, $get_method:ident, $text:expr) => {{
        let response = $ui.checkbox(&mut $reg.$get_method(), $text);
        $ui.separator();

        if response.changed() {
            $reg.$set_method(!$reg.$get_method());
        }

        response.changed()
    }};
}

macro_rules! slider {
    ($ui:expr, $reg:expr, $set_method:ident, $get_method:ident, $text:expr, $range:expr) => {{
        let mut current_val = $reg.$get_method();
        let response = egui::Slider::new(&mut current_val, $range).text($text).ui($ui);
        $ui.separator();

        if response.changed() {
            $reg.$set_method(current_val);
        }

        response.changed()
    }};
}

pub struct DispCntView;
pub struct DispStatView;
pub struct BgControlView;
pub struct BgScrollView;

impl ViewableRegister for DispCntView {
    fn get_name(&self) -> &'static str {
        "DispCnt"
    }

    fn get_current_value(&self, reg_value: &[u8]) -> String {
        format!("{:#06X}", u16::from_le_bytes(reg_value.try_into().unwrap()))
    }

    fn get_address(&self, instance: u32) -> RangeInclusive<MemoryAddress> {
        LCD_CONTROL_START..=LCD_CONTROL_END
    }

    fn draw(&self, ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
        let mut reg = grba_core::emulator::debug::LcdControl::from_le_bytes(reg_value.try_into().unwrap());
        let mut changed = false;

        changed |= {
            let mut current_val = reg.bg_mode();

            let changed = egui::ComboBox::new("bg_mode_combo", "[0x0..=0x2] Bg Mode")
                .selected_text(format!("{:?}", current_val))
                .show_ui(ui, |ui| {
                    let mut changed = false;

                    for variant in BgMode::into_enum_iter() {
                        changed |= ui
                            .selectable_value(&mut current_val, variant, format!("{:?}", variant))
                            .changed();
                    }

                    changed
                })
                .inner
                .unwrap_or_default();
            ui.separator();

            if changed {
                reg.set_bg_mode(current_val);
            }

            changed
        };

        changed |= checkbox!(
            ui,
            reg,
            set_display_frame_select,
            display_frame_select,
            "[0x4] Display Frame Select (BG-Modes 4,5 only)"
        );

        changed |= checkbox!(
            ui,
            reg,
            set_h_blank_interval_free,
            h_blank_interval_free,
            "[0x5] H-Blank Interval Free (Allow access to OAM during H-Blank)"
        );

        changed |= checkbox!(
            ui,
            reg,
            set_obj_character_vram_mapping,
            obj_character_vram_mapping,
            "[0x6] OBJ Character VRAM Mapping (0=Two dimensional, 1=One dimensional)"
        );

        changed |= checkbox!(
            ui,
            reg,
            set_forced_blank,
            forced_blank,
            "[0x7] Forced blank (1=Allow FAST access to VRAM,Palette,OAM)"
        );

        changed |= checkbox!(
            ui,
            reg,
            set_screen_display_bg0,
            screen_display_bg0,
            "[0x8] Screen Display BG0"
        );

        changed |= checkbox!(
            ui,
            reg,
            set_screen_display_bg1,
            screen_display_bg1,
            "[0x9] Screen Display BG1"
        );

        changed |= checkbox!(
            ui,
            reg,
            set_screen_display_bg2,
            screen_display_bg2,
            "[0xA] Screen Display BG2"
        );

        changed |= checkbox!(
            ui,
            reg,
            set_screen_display_bg3,
            screen_display_bg3,
            "[0xB] Screen Display BG3"
        );

        changed |= checkbox!(
            ui,
            reg,
            set_screen_display_obj,
            screen_display_obj,
            "[0xC] Screen Display OBJ"
        );

        changed |= checkbox!(
            ui,
            reg,
            set_window_0_display_flag,
            window_0_display_flag,
            "[0xD] Window 0 Display Flag"
        );

        changed |= checkbox!(
            ui,
            reg,
            set_window_1_display_flag,
            window_1_display_flag,
            "[0xE] Window 1 Display Flag"
        );

        changed |= checkbox!(
            ui,
            reg,
            set_obj_window_display,
            obj_window_display,
            "[0xF] OBJ Window Display Flag"
        );

        if changed {
            Some(reg.to_le_bytes().into())
        } else {
            None
        }
    }
}

impl ViewableRegister for DispStatView {
    fn get_name(&self) -> &'static str {
        "DispStat"
    }

    fn get_current_value(&self, reg_value: &[u8]) -> String {
        format!("{:#06X}", u16::from_le_bytes(reg_value.try_into().unwrap()))
    }

    fn get_address(&self, instance: u32) -> RangeInclusive<MemoryAddress> {
        LCD_STATUS_START..=LCD_STATUS_END
    }

    fn draw(&self, ui: &mut Ui, reg_value: &[u8]) -> Option<Vec<u8>> {
        let mut reg = grba_core::emulator::debug::LcdStatus::from_le_bytes(reg_value.try_into().unwrap());
        let mut changed = false;

        changed |= checkbox!(ui, reg, set_v_blank_flag, v_blank_flag, "[0x0] V Blank Flag");
        changed |= checkbox!(ui, reg, set_h_blank_flag, h_blank_flag, "[0x1] H Blank Flag");
        changed |= checkbox!(ui, reg, set_v_counter_flag, v_counter_flag, "[0x2] V Counter Flag");
        changed |= checkbox!(
            ui,
            reg,
            set_v_blank_irq_enable,
            v_blank_irq_enable,
            "[0x3] V Blank IRQ Enable"
        );
        changed |= checkbox!(
            ui,
            reg,
            set_h_blank_irq_enable,
            h_blank_irq_enable,
            "[0x4] H Blank IRQ Enable"
        );
        changed |= checkbox!(
            ui,
            reg,
            set_v_counter_irq_enable,
            v_counter_irq_enable,
            "[0x5] V Counter IRQ Enable"
        );

        changed |= slider!(
            ui,
            reg,
            set_v_count_setting_lyc,
            v_count_setting_lyc,
            "[0x8..=0xF] V Count Setting LYC",
            0..=255
        );

        if changed {
            Some(reg.to_le_bytes().into())
        } else {
            None
        }
    }
}
