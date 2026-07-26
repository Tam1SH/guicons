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

pub fn icon_from_path(path: &str) -> windows_reactor::Icon {
    windows_reactor::Icon::bitmap(uri_from_path(path))
}
