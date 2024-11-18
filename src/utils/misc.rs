use std::ops::{Add, Rem, Sub};
use std::sync::{Arc, Mutex};

use tui::style::Color;

// pub type

pub type Shared<T> = Arc<Mutex<T>>;

pub fn create_shared<T>(value_to_share: T) -> Shared<T> {
    Arc::new(Mutex::new(value_to_share))
}

pub fn shared_copy<T>(value_to_copy: &Shared<T>) -> Shared<T> {
    Arc::clone(value_to_copy)
}

pub fn remainder<T: Add + Sub + Rem + Copy>(
    dividend: T,
    divisor: T,
) -> <<<<T as Add>::Output as Rem<T>>::Output as Add<T>>::Output as Rem<T>>::Output
where
    <T as Add>::Output: Rem<T>,
    <<T as Add>::Output as Rem<T>>::Output: Add<T>,
    <<<T as Add>::Output as Rem<T>>::Output as Add<T>>::Output: Rem<T>,
{
    (((dividend + divisor) % divisor) + divisor) % divisor
}

pub fn hex_to_rgb(hex: &str) -> Result<Color, String> {
    let hex = hex.trim_start_matches("#");

    if hex.len() != 6 {
        return Err("Hex code must be 6 characters long".to_string());
    }

    let red = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid red component")?;
    let green = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid green component")?;
    let blue = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid blue component")?;

    Ok(Color::Rgb(red, green, blue))
}

pub fn wrapped_iter_enumerate<T>(vec: &Vec<T>, start: usize) -> impl Iterator<Item = (usize, &T)> {
    let len = vec.len();
    (0..len).map(move |i| {
        let index = (start + i) % len;
        (index, &vec[index])
    })
}

pub const WIK_TITLE: &str = r"
▀██ ▀██ ▀█▀ ▀█▀  ██  ▀██     
  ██  ███   █   ▄▄▄   ██  ▄▄ 
   ██  ██  █     ██   ██ ▄▀  
    ███ ███      ██   ██▀█▄  
     █   █      ▄██▄ ▄██▄ ██▄";
