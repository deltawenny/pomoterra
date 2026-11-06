//! Lucide's typed catalog and the application's single icon renderer.
//!
//! Keep glyphs paired with the font from the same dependency version.
pub use lucide_icons::Icon;
use nsh_core::element::{AssetFile, AssetKind};
use nsh_core::prelude::*;

/// NSH serves and packages this font for generated runners and TSX entries.
pub const ASSETS: &[AssetFile] = &[AssetFile {
    uri: "asset:/@nsh-app-pomoterra/lucide.ttf",
    source: concat!(env!("OUT_DIR"), "/lucide.ttf"),
    kind: AssetKind::Font,
}];

/// A decorative icon in a fixed square, sized in logical pixels.
/// Put the accessible action name on the enclosing control.
pub fn icon(name: Icon, color: u32, size: f32) -> View {
    view()
        .w(px(size))
        .h(px(size))
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .accessibility_elements_hidden(true)
        .child(
            text(char::from(name).to_string())
                .font_family("lucide")
                .text_size(px(size))
                .text_color(rgb(color))
                .allow_font_scaling(false),
        )
}
