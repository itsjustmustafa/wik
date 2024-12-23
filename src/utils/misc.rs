use std::ops::{Add, Rem, Sub};
use std::sync::{Arc, Mutex};

use ratatui::style::Color;

pub const WIK_TITLE: &str = r"
▀██ ▀██ ▀█▀ ▀█▀  ██  ▀██     
  ██  ███   █   ▄▄▄   ██  ▄▄ 
   ██  ██  █     ██   ██ ▄▀  
    ███ ███      ██   ██▀█▄  
     █   █      ▄██▄ ▄██▄ ██▄";

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

pub fn blend_color_value(a: u8, b: u8, t: u8) -> u8 {
    let norm_t = (t as f64) / 100.0;
    let a_squared = (a as f64).powi(2);
    let b_squared = (b as f64).powi(2);
    let blended_value = ((1.0 - norm_t) * a_squared + norm_t * b_squared).sqrt();
    return blended_value.round() as u8;
}

pub fn blended_color(base_color: Color, blend_color: Color, alpha: u8) -> Color {
    match try_color_as_rgb(base_color) {
        Color::Rgb(r1, g1, b1) => match try_color_as_rgb(blend_color) {
            Color::Rgb(r2, g2, b2) => {
                // return blend_color;
                return Color::Rgb(
                    blend_color_value(r1, r2, alpha),
                    blend_color_value(g1, g2, alpha),
                    blend_color_value(b1, b2, alpha),
                );
            }
            _ => {
                return base_color;
            }
        },
        _ => {
            return base_color;
        }
    }
}

pub fn try_color_as_rgb(color: Color) -> Color {
    match color {
        Color::Black => return Color::Rgb(0, 0, 0),
        Color::Red => return Color::Rgb(205, 49, 49),
        Color::Green => return Color::Rgb(13, 188, 121),
        Color::Yellow => return Color::Rgb(229, 229, 16),
        Color::Blue => return Color::Rgb(36, 114, 200),
        Color::Magenta => return Color::Rgb(188, 63, 188),
        Color::Cyan => return Color::Rgb(17, 168, 205),
        Color::Gray => return Color::Rgb(102, 102, 102),
        Color::DarkGray => return Color::Rgb(63, 63, 63),
        Color::LightRed => return Color::Rgb(241, 76, 76),
        Color::LightGreen => return Color::Rgb(35, 209, 139),
        Color::LightYellow => return Color::Rgb(245, 245, 67),
        Color::LightBlue => return Color::Rgb(59, 142, 234),
        Color::LightMagenta => return Color::Rgb(214, 112, 214),
        Color::LightCyan => return Color::Rgb(41, 184, 219),
        Color::White => return Color::Rgb(229, 229, 229),
        Color::Rgb(r, g, b) => return Color::Rgb(r, g, b),
        _ => {}
    }

    return color;
}

pub fn wrapped_iter_enumerate<T>(vec: &Vec<T>, start: usize) -> impl Iterator<Item = (usize, &T)> {
    let len = vec.len();
    (0..len).map(move |i| {
        let index = (start + i) % len;
        (index, &vec[index])
    })
}

pub fn cut_off_from_char(text: &str, delimiter: char) -> &str {
    text.splitn(2, delimiter).next().unwrap_or(&text).trim()
}
