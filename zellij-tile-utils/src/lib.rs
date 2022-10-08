#[macro_export]
macro_rules! rgb {
    ($a:expr) => {
        ansi_term::Color::Rgb($a.0, $a.1, $a.2)
    };
}

#[macro_export]
macro_rules! palette_match {
    ($palette_color:expr) => {
        match $palette_color {
            PaletteColor::Rgb((r, g, b)) => RGB(r, g, b),
            PaletteColor::EightBit(color) => Fixed(color),
        }
    };
}

#[macro_export]
macro_rules! style {
    ($fg:expr, $bg:expr) => {
        match ($fg, $bg) {
            (PaletteColor::Transparent, PaletteColor::Transparent) => ansi_term::Style::new(),
            (PaletteColor::Transparent, PaletteColor::Rgb((r, g, b))) => {
                ansi_term::Style::new().on(ansi_term::Color::RGB(r, g, b))
            },
            (PaletteColor::Transparent, PaletteColor::EightBit(color)) => {
                ansi_term::Style::new().on(ansi_term::Color::Fixed(color))
            },
            (PaletteColor::Rgb((r, g, b)), PaletteColor::Transparent) => {
                ansi_term::Style::new().fg(ansi_term::Color::RGB(r, g, b))
            },
            (PaletteColor::Rgb((r, g, b)), PaletteColor::Rgb((r2, g2, b2))) => {
                ansi_term::Style::new()
                    .fg(ansi_term::Color::RGB(r, g, b))
                    .on(ansi_term::Color::RGB(r2, g2, b2))
            },
            (PaletteColor::Rgb((r, g, b)), PaletteColor::EightBit(color)) => {
                ansi_term::Style::new()
                    .fg(ansi_term::Color::RGB(r, g, b))
                    .on(ansi_term::Color::Fixed(color))
            },
            (PaletteColor::EightBit(color), PaletteColor::Transparent) => {
                ansi_term::Style::new().fg(ansi_term::Color::Fixed(color))
            },
            (PaletteColor::EightBit(color), PaletteColor::Rgb((r2, g2, b2))) => {
                ansi_term::Style::new()
                    .fg(ansi_term::Color::Fixed(color))
                    .on(ansi_term::Color::RGB(r2, g2, b2))
            },
            (PaletteColor::EightBit(color), PaletteColor::EightBit(color2)) => {
                ansi_term::Style::new()
                    .fg(ansi_term::Color::Fixed(color))
                    .on(ansi_term::Color::Fixed(color2))
            },
        }
    };
}
