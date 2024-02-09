# Themes

The default themes provided are the [Catppuccin](https://github.com/catppuccin/catppuccin) flavors.

## User themes

There is now also support for custom user themes.

For this, simply create a folder `.config/loungy/themes`.

There create a `.toml` file with the following structure:

```rust
name = "Your Theme Name"
font_sans = "Inter"
font_mono = "JetBrains Mono"
flamingo = "#FF77A8"
pink = "#FF2281"
mauve = "#E0B0FF"
red = "#FF0000"
maroon = "#800000"
peach = "#FFE5B4"
yellow = "#FFFF00"
green = "#008000"
teal = "#008080"
sky = "#87CEEB"
sapphire = "#0F52BA"
blue = "#0000FF"
lavender = "#E6E6FA"
text = "#333333"
subtext1 = "#666666"
subtext0 = "#999999"
overlay2 = "#CCCCCC"
overlay1 = "#DDDDDD"
overlay0 = "#EEEEEE"
surface2 = "#F2F2F2"
surface1 = "#F7F7F7"
surface0 = "#FFFFFF"
base = "#B8B8B8"
mantle = "#A0A0A0"
crust = "#8B8B8B"
```

The colors support both `#RRGGBB` and `#RRGGBBAA` format.

When you restart Loungy you can then go to the `Search Themes` command and make your custom theme the new default!

![Theme selection](../img/theme.png)

**Important**: I don't guarantee that this format will remain supported as it's still quite early in the development.
