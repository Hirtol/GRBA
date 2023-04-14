use egui::Ui;

pub fn text_edit_uint(ui: &mut Ui, text: &mut String, hint: &str, radix: u32) -> Option<u64> {
    let response = ui.add(egui::TextEdit::singleline(text).hint_text(hint));

    text.retain(|c| c.is_digit(radix));

    // For some reason egui is triggering response.clicked() when we press enter at the moment
    // (didn't used to do this). The additional check for not having enter pressed will need to stay until that is fixed.
    if response.clicked() && !ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        text.clear();
    }

    // If we pressed enter, move to the address
    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        u64::from_str_radix(text, 10).ok()
    } else {
        None
    }
}
