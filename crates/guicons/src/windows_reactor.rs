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

pub fn icon_from_path(path: &str, width: f64, height: f64) -> windows_reactor::Icon {
    let uri = uri_from_path(path);
    if path.to_ascii_lowercase().ends_with(".svg") {
        windows_reactor::Icon::svg(uri, width, height)
    } else {
        windows_reactor::Icon::bitmap(uri)
    }
}

/// What `icon!(...)` expands to under the `windows-reactor` feature: an icon
/// with the source resolved, but the size left to the use site. Finishes via
/// `.build()` or anywhere `Into<windows_reactor::Icon>` is accepted.
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

    pub fn build(self) -> windows_reactor::Icon {
        icon_from_path(
            &self.path,
            self.width.unwrap_or(DEFAULT_ICON_SIZE),
            self.height.unwrap_or(DEFAULT_ICON_SIZE),
        )
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
