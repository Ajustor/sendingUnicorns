use home::home_dir;

pub fn get() -> String {
    let home = home_dir().unwrap().display().to_string();
    format!("{home}/.sending_unicorns")
}
