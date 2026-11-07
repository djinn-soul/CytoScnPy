/// Main CSS stylesheet for the HTML report, including the dark and light
/// theme tokens and the print stylesheet.
pub const STYLE_CSS: &str = include_str!("assets/style.css");

/// CSS for the `Prism.js` code highlighter, driven by the report theme tokens.
pub const PRISM_CSS: &str = include_str!("assets/prism.css");

/// Theme bootstrap and toggle script. The `THEME_DEFAULT_PLACEHOLDER` token is
/// replaced with the configured default theme when the report is written.
pub const THEME_JS: &str = include_str!("assets/theme.js");

/// Placeholder in [`THEME_JS`] substituted with the default theme name.
pub const THEME_DEFAULT_PLACEHOLDER: &str = "__CYTOSCNPY_DEFAULT_THEME__";

/// `JavaScript` for the `Prism.js` code highlighter and its plugins.
pub const PRISM_JS: &str = include_str!("assets/prism.js");
