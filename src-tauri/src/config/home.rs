pub fn get() -> String {
    if let Some(home) = dirs::data_dir().unwrap().to_str() {
        return format!("{home}/.sending_unicorns");
    } else {
        return "".to_string();
    }
}
