use std::path::{Path, PathBuf};
use std::env;

// TODO: This needs a lot of work, cross platform and where the game data is located
// Also versioning stuff


// ASSSETS
pub const ASSET_FILENAME_HOGTYPE_D3: &str = "d3.hog";

pub fn get_game_dir_path() -> PathBuf {
    Path::new(&env::var("HOME").unwrap()).join("Descent3")
}

pub fn get_asset_path(path: &str) -> PathBuf {
    get_game_dir_path().join(path)
}

#[cfg(test)]
pub mod testing {
    use std::{fs::File, io::BufReader, path::PathBuf};

    use crate::filesystem::hog::Hog;
    use anyhow::{Context, Result};

    use super::{get_asset_path, ASSET_FILENAME_HOGTYPE_D3};

    pub fn get_d3_hog() -> Result<Hog> {
        let path = get_asset_path(ASSET_FILENAME_HOGTYPE_D3);
        let mut hogfile = File::open(path).unwrap();
        let mut reader = BufReader::new(hogfile);

        Hog::new_from_stream(&mut reader, ASSET_FILENAME_HOGTYPE_D3.to_string()).context("Failed to open d3 hog")
    }

}