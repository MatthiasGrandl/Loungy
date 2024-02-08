use gpui::*;

fn color_to_hsla(color: catppuccin::Colour) -> Hsla {
    Rgba {
        r: color.0 as f32 / 255.0,
        g: color.1 as f32 / 255.0,
        b: color.2 as f32 / 255.0,
        a: 1.0,
    }
    .into()
}

impl From<catppuccin::FlavourColours> for Theme {
    fn from(colors: catppuccin::FlavourColours) -> Self {
        Self {
            font_sans: "Inter".into(),
            font_mono: "JetBrains Mono".into(),
            flamingo: color_to_hsla(colors.flamingo),
            pink: color_to_hsla(colors.pink),
            mauve: color_to_hsla(colors.mauve),
            red: color_to_hsla(colors.red),
            maroon: color_to_hsla(colors.maroon),
            peach: color_to_hsla(colors.peach),
            yellow: color_to_hsla(colors.yellow),
            green: color_to_hsla(colors.green),
            teal: color_to_hsla(colors.teal),
            sky: color_to_hsla(colors.sky),
            sapphire: color_to_hsla(colors.sapphire),
            blue: color_to_hsla(colors.blue),
            lavender: color_to_hsla(colors.lavender),
            text: color_to_hsla(colors.text),
            subtext1: color_to_hsla(colors.subtext1),
            subtext0: color_to_hsla(colors.subtext0),
            overlay2: color_to_hsla(colors.overlay2),
            overlay1: color_to_hsla(colors.overlay1),
            overlay0: color_to_hsla(colors.overlay0),
            surface2: color_to_hsla(colors.surface2),
            surface1: color_to_hsla(colors.surface1),
            surface0: color_to_hsla(colors.surface0),
            base: color_to_hsla(colors.base),
            mantle: color_to_hsla(colors.mantle),
            crust: color_to_hsla(colors.crust),
        }
    }
}

#[derive(Debug)]
pub struct Theme {
    pub font_sans: SharedString,
    pub font_mono: SharedString,
    pub flamingo: Hsla,
    pub pink: Hsla,
    pub mauve: Hsla,
    pub red: Hsla,
    pub maroon: Hsla,
    pub peach: Hsla,
    pub yellow: Hsla,
    pub green: Hsla,
    pub teal: Hsla,
    pub sky: Hsla,
    pub sapphire: Hsla,
    pub blue: Hsla,
    pub lavender: Hsla,
    pub text: Hsla,
    pub subtext1: Hsla,
    pub subtext0: Hsla,
    pub overlay2: Hsla,
    pub overlay1: Hsla,
    pub overlay0: Hsla,
    pub surface2: Hsla,
    pub surface1: Hsla,
    pub surface0: Hsla,
    pub base: Hsla,
    pub mantle: Hsla,
    pub crust: Hsla,
}

fn load_fonts(cx: &mut AppContext) -> gpui::Result<()> {
    let font_paths = cx.asset_source().list("fonts")?;
    let mut embedded_fonts = Vec::new();
    for font_path in font_paths {
        if font_path.ends_with(".ttf") {
            let font_bytes = cx.asset_source().load(&font_path)?;
            embedded_fonts.push(font_bytes);
        }
    }
    cx.text_system().add_fonts(embedded_fonts)
}

impl Theme {
    pub fn init(cx: &mut AppContext) {
        load_fonts(cx).expect("Failed to load fonts");
        cx.set_global(Theme::new())
    }

    pub fn change(flavor: catppuccin::Flavour, cx: &mut WindowContext) {
        cx.set_global(Self::from(flavor.colours()));
        cx.refresh();
    }

    fn new() -> Self {
        Self::from(catppuccin::Flavour::Mocha.colours())
    }
}

impl Global for Theme {}
