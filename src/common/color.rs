use crate::common::types::Color;

pub fn new_color(rgb: (u8, u8, u8)) -> Color {
    let (r, g, b) = rgb;
    Color {
        r: r as u32,
        g: g as u32,
        b: b as u32,
    }
}

fn black() -> Color {
    Color { r: 0, g: 0, b: 0 }
}

fn white() -> Color {
    Color {
        r: 255,
        g: 255,
        b: 255,
    }
}

pub fn get_rgb_from_hex(color: &str) -> Result<Color, std::num::ParseIntError> {
    let red = u8::from_str_radix(&color[0..2], 16)?;
    let green = u8::from_str_radix(&color[2..4], 16)?;
    let blue = u8::from_str_radix(&color[4..6], 16)?;

    Ok(new_color((red, green, blue)))
}

pub fn get_luminance(color: &Color) -> f64 {
    let red = color.r as u8;
    let green = color.g as u8;
    let blue = color.b as u8;

    let new_value = |c: u8| -> f64 {
        let c = c as f64 / 255.0;

        // I forget where this magic came from but it worked well enough lol
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };

    let r = new_value(red);
    let g = new_value(green);
    let b = new_value(blue);

    // This is kind of cool -- it relates human eye sensitivity to
    // red green and blue respectively, weighting the "brightness" of each
    // color component. Neat!
    (0.2126 * r) + (0.7152 * g) + (0.0722 * b)
}

pub fn get_contrast(primary: &Color, secondary: &Color) -> f64 {
    let primary_luminance = get_luminance(primary);
    let secondary_luminance = get_luminance(secondary);

    let max = primary_luminance.max(secondary_luminance);
    let min = primary_luminance.min(secondary_luminance);

    (max + 0.05) / (min + 0.05)
}

// Returns a valid secondary color to go with the provided primary
pub fn get_secondary_for_primary<'a>(
    primary: &'a str,
    secondary_candidate: &'a str,
) -> Result<Color, std::num::ParseIntError> {
    let primary = get_rgb_from_hex(primary)?;
    let secondary_color = get_rgb_from_hex(secondary_candidate)?;

    let contrast = get_contrast(&primary, &secondary_color);

    let white_contrast = get_contrast(&primary, &white());
    let black_contrast = get_contrast(&primary, &black());

    if contrast > 3.5 {
        Ok(secondary_color)
    } else if white_contrast > black_contrast {
        Ok(white())
    } else {
        Ok(black())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() -> Result<(), std::num::ParseIntError> {
        let result = get_secondary_for_primary("de3129", "666666")?;
        assert_eq!(result, white());
        Ok(())
    }
}
