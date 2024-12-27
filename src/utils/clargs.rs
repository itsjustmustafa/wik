use std::{
    fs::{self, File},
    io::{self, BufReader},
};

use clap::Parser;
use dirs::home_dir;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Parser, Serialize, Deserialize)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Search query for Wikipedia page (eg. hotdogs)
    #[arg(short, long)]
    pub search: Option<String>,
    /// Name of specific page to be loaded
    #[arg(short, long)]
    pub page: Option<String>,
    /// Number of rows for display (default to None - gets terminal's rows)
    #[arg(short, long)]
    pub rows: Option<u16>,
    /// Number of columns for display (default to None - gets terminal's columns)
    #[arg(short, long)]
    pub cols: Option<u16>,
    /// Margin size of application (defaults to no margin)
    #[arg(short, long, default_value_t = 0)]
    pub margin: u16,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            search: None,
            page: None,
            rows: None,
            cols: None,
            margin: 0,
        }
    }
}

impl Args {
    pub fn is_default_configs(&self) -> bool {
        self.rows.is_none() && self.cols.is_none() && (self.margin == 0)
    }

    pub fn load_from(&mut self, other: Args) {
        self.rows = other.rows;
        self.cols = other.cols;
        self.margin = other.margin;
    }
}

const CONFIG_SAVE_FILE: &str = ".config/wik/config.json";

pub fn load_arg_from_config() -> Option<Args> {
    if let Some(home_dir_path) = home_dir() {
        let file_path = home_dir_path.join(CONFIG_SAVE_FILE);
        let file_result = File::options().read(true).write(false).open(file_path);
        match file_result {
            Ok(file) => {
                let reader = BufReader::new(file);
                match serde_json::from_reader::<BufReader<File>, Args>(reader) {
                    Ok(new_args) => Some(new_args),
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    } else {
        None
    }
}

pub fn save_arg_to_file(args: &Args) -> io::Result<()> {
    let serialized_args = serde_json::to_string_pretty(args).unwrap_or(String::from(""));
    if let Some(home_dir_path) = home_dir() {
        let file_path = home_dir_path.join(CONFIG_SAVE_FILE);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(file_path, serialized_args)?;
    }
    Ok(())
}
