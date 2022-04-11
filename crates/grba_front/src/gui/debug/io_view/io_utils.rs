use egui::{Ui, Widget};
use grba_core::utils::BitOps;
use std::ops::RangeInclusive;

pub fn io_list(
    ui: &mut Ui,
    register_contents: &mut u32,
    bit_range: RangeInclusive<u8>,
    name: &str,
    items: &[impl AsRef<str>],
) -> bool {
    let mut current_val = register_contents.get_bits(*bit_range.start(), *bit_range.end());

    let full_name = create_full_name(name, &bit_range);

    let changed = egui::ComboBox::new(name, full_name)
        .selected_text(items[current_val as usize].as_ref())
        .show_ui(ui, |ui| {
            let mut changed = false;

            for (i, variant) in items.iter().enumerate() {
                changed |= ui
                    .selectable_value(&mut current_val, i as u32, variant.as_ref())
                    .changed();
            }

            changed
        })
        .inner
        .unwrap_or_default();

    ui.separator();

    if changed {
        *register_contents = register_contents.change_bits(*bit_range.start(), *bit_range.end(), current_val);
    }

    changed
}

pub fn io_checkbox(ui: &mut Ui, register_contents: &mut u32, bit: u8, name: &str) -> bool {
    let mut current_val = register_contents.check_bit(bit);

    let full_name = create_full_name(name, &(bit..=bit));

    let changed = ui.checkbox(&mut current_val, full_name).changed();

    ui.separator();

    if changed {
        *register_contents = register_contents.change_bits(bit, bit, current_val as u32);
    }

    changed
}

pub fn io_radio(ui: &mut Ui, register_contents: &mut u32, bit_range: RangeInclusive<u8>, name: &str) -> bool {
    let mut current_val = register_contents.get_bits(*bit_range.start(), *bit_range.end());

    let full_name = create_full_name(name, &bit_range);

    ui.label(full_name);

    let changed = ui
        .horizontal(|ui| {
            let mut changed = false;

            for i in 0..(2u32.pow(bit_range.size_hint().0 as u32)) {
                changed |= ui.radio_value(&mut current_val, i as u32, format!("{}", i)).changed();
            }

            changed
        })
        .inner;

    ui.separator();

    if changed {
        *register_contents = register_contents.change_bits(*bit_range.start(), *bit_range.end(), current_val);
    }

    changed
}

pub fn io_slider(
    ui: &mut Ui,
    register_contents: &mut u32,
    bit_range: RangeInclusive<u8>,
    name: &str,
    slider_range: RangeInclusive<u32>,
) -> bool {
    let mut current_val = register_contents.get_bits(*bit_range.start(), *bit_range.end());

    let full_name = create_full_name(name, &bit_range);

    let changed = egui::Slider::new(&mut current_val, slider_range)
        .text(full_name)
        .ui(ui)
        .changed();

    ui.separator();

    if changed {
        *register_contents = register_contents.change_bits(*bit_range.start(), *bit_range.end(), current_val);
    }

    changed
}

fn create_full_name(name: &str, bit_range: &RangeInclusive<u8>) -> String {
    if bit_range.size_hint().0 == 1 {
        format!("[{:#X}] {}", bit_range.start(), name)
    } else {
        format!("[{:#X}..={:#X}] {}", bit_range.start(), bit_range.end(), name)
    }
}
