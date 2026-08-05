pub const DEFAULT_ICON_SIZE: f64 = 16.0;

pub fn uri_from_path(path: &str) -> String {
    if path.contains("://") {
        path.to_string()
    } else {
        format!("file:///{}", path.replace('\\', "/"))
    }
}

pub fn image_from_path(path: &str) -> windows_reactor::Image {
    windows_reactor::Image::new_with_uri(uri_from_path(path))
}

/// Size-less: an `IconElement` slot (button/nav-item icon, ...) has no place
/// to apply a width/height anyway - sizing only matters for the
/// [`icon_builder`]/[`IconBuilder::build_element`] path, which returns a
/// plain sized `Image` instead of going through `Icon` at all.
pub fn icon_from_path(path: &str) -> windows_reactor::Icon {
    windows_reactor::Icon::image(uri_from_path(path))
}

/// What `icon!(...)` expands to under the `windows-reactor` feature: an icon
/// with the source resolved, sizing applied only if the call site actually
/// wants a standalone sized image ([`IconBuilder::build_element`]) - `.build()`
/// for an `IconElement` slot ignores it, since `windows_reactor::Icon` has no
/// size field to put it in.
pub struct IconBuilder {
    path: String,
    width: Option<f64>,
    height: Option<f64>,
}

pub fn icon_builder(path: impl Into<String>) -> IconBuilder {
    IconBuilder { path: path.into(), width: None, height: None }
}

impl IconBuilder {
    pub fn size(mut self, size: f64) -> Self {
        self.width = Some(size);
        self.height = Some(size);
        self
    }

    pub fn width(mut self, width: f64) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: f64) -> Self {
        self.height = Some(height);
        self
    }

    /// For an `IconElement` slot (button/nav-item `.icon(...)`) - no size
    /// applied, `windows_reactor::Icon` has nowhere to put it.
    pub fn build(self) -> windows_reactor::Icon {
        icon_from_path(&self.path)
    }

    /// For a standalone, explicitly-sized icon (a table cell, a custom
    /// layout) - bypasses `Icon` entirely and returns a real `Image` with
    /// `.width()/.height()` already applied.
    pub fn build_element(self) -> windows_reactor::Element {
        use windows_reactor::ElementExt;
        windows_reactor::Element::from(image_from_path(&self.path))
            .width(self.width.unwrap_or(DEFAULT_ICON_SIZE))
            .height(self.height.unwrap_or(DEFAULT_ICON_SIZE))
    }
}

impl From<IconBuilder> for windows_reactor::Icon {
    fn from(builder: IconBuilder) -> Self {
        builder.build()
    }
}

pub fn glyph_icon(font_family: &str, codepoint: char) -> windows_reactor::Icon {
    windows_reactor::Icon::Font { glyph: codepoint.to_string(), family: Some(font_family.to_string()) }
}
