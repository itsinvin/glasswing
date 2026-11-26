//! The one place colours live.
//!
//! Glasswing is a dark app in version 1, so there is a single theme. Colours are written in
//! OKLCH, which keeps greys neutral at every lightness and matches how Rune blends light.

use rune::{oklch, oklcha, Color};

/// Everything pages draw with.
#[derive(Clone, Copy)]
pub struct Theme {
    /// The window background.
    pub bg: Color,
    /// The sidebar, a step darker than the page.
    pub sidebar: Color,
    /// A raised panel.
    pub card: Color,
    /// A card's hairline edge.
    pub line: Color,
    /// Fill for buttons, fields and inset wells.
    pub well: Color,
    /// A hover overlay, painted over a fill.
    pub hover: Color,
    /// A press overlay, painted over a fill.
    pub press: Color,
    /// Primary text.
    pub text: Color,
    /// Secondary text.
    pub dim: Color,
    /// Hints, placeholders and small print.
    pub faint: Color,
    /// The accent colour: links, active rows, the bitrate fill.
    pub accent: Color,
    /// A pale accent for tints behind a chosen thing.
    pub accent_tint: Color,
    /// "All good" green.
    pub good: Color,
    /// "Check this" amber.
    pub warn: Color,
    /// "Something is wrong" red.
    pub bad: Color,
    /// A slider or switch track.
    pub track: Color,
    /// Drop shadows.
    pub shadow: Color,
}

impl Theme {
    pub fn dark() -> Theme {
        Theme {
            bg: oklch(0.176, 0.006, 265.0),
            sidebar: oklch(0.150, 0.006, 265.0),
            card: oklch(0.228, 0.007, 265.0),
            line: oklcha(1.0, 0.0, 0.0, 0.07),
            well: oklcha(1.0, 0.0, 0.0, 0.05),
            hover: oklcha(1.0, 0.0, 0.0, 0.05),
            press: oklcha(1.0, 0.0, 0.0, 0.10),
            text: oklch(0.955, 0.003, 265.0),
            dim: oklch(0.740, 0.008, 265.0),
            faint: oklch(0.560, 0.010, 265.0),
            accent: oklch(0.660, 0.170, 285.0),
            accent_tint: oklcha(0.660, 0.170, 285.0, 0.22),
            good: oklch(0.760, 0.140, 155.0),
            warn: oklch(0.800, 0.140, 78.0),
            bad: oklch(0.660, 0.190, 25.0),
            track: oklcha(1.0, 0.0, 0.0, 0.10),
            shadow: oklcha(0.0, 0.0, 0.0, 0.45),
        }
    }
}

/// Type sizes. Named for what they do, not how big they are.
pub mod type_ {
    use rune::tok::TextStyle;

    /// Page titles.
    pub const H1: TextStyle = TextStyle::new(20.0, 650.0);
    /// Card titles.
    pub const H2: TextStyle = TextStyle::new(14.0, 600.0);
    /// Body text and field text.
    pub const BODY: TextStyle = TextStyle::new(13.0, 400.0);
    /// Console names and other emphasised body text.
    pub const BODY_STRONG: TextStyle = TextStyle::new(13.0, 550.0);
    /// Labels above fields, buttons and nav rows.
    pub const LABEL: TextStyle = TextStyle::new(12.0, 500.0);
    /// Hints, help text and small print.
    pub const SMALL: TextStyle = TextStyle::new(11.5, 400.0);
    /// Numbers, IDs and paths.
    pub const MONO: TextStyle = TextStyle::new(11.5, 500.0).mono();
}
