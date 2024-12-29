use std::{fs, path::PathBuf};
pub fn get_working_dir() -> PathBuf {
    let base_path = dirs_next::data_dir().expect("Could not obtain data directory");

    let base_path = base_path.join("youterm");
    fs::create_dir_all(&base_path).expect("failed to create data directory");

    base_path
}
pub fn get_bin_dir() -> PathBuf {
    let base_path = dirs_next::data_dir().expect("Could not obtain data directory");

    let base_path = base_path.join("youterm");
    let base_path = base_path.join("bin");
    fs::create_dir_all(&base_path).expect("failed to create data directory");

    base_path
}

pub fn get_out_dir() -> PathBuf {
    let base_path = dirs_next::data_dir().expect("Could not obtain data directory");

    let base_path = base_path.join("youterm");
    let base_path = base_path.join("out");
    fs::create_dir_all(&base_path).expect("failed to create data directory");

    base_path
}