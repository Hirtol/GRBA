use egui::Ui;

pub enum TextInput {
    Relative,
    Absolute,
}

impl TextInput {
    pub fn is_relative(&self) -> bool {
        matches!(self, TextInput::Relative)
    }
}

pub fn text_edit_uint(ui: &mut Ui, text: &mut String, hint: &str, hover: &str, radix: u32) -> Option<(TextInput, u64)> {
    let response = ui.add(egui::TextEdit::singleline(text).hint_text(hint)).on_hover_text(hover);

    text.retain(|c| c.is_digit(radix) || c == '#');

    // For some reason egui is triggering response.clicked() when we press enter at the moment
    // (didn't used to do this). The additional check for not having enter pressed will need to stay until that is fixed.
    if response.clicked() && !ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        text.clear();
    }

    // If we pressed enter, move to the address
    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        let (input, text) = text
            .strip_prefix('#')
            .map(|i| (TextInput::Relative, i))
            .unwrap_or((TextInput::Absolute, &text));

        Some((input, u64::from_str_radix(text, radix).ok()?))
    } else {
        None
    }
}
