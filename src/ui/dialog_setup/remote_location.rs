use crate::ui::prelude::*;

pub struct RemoteLocation {
    url: String,
}

impl RemoteLocation {
    pub fn from_user_input(input: String) -> std::result::Result<Self, String> {
        let url = if !input.contains("://") {
            if let Some((target, path)) = input.split_once(":") {
                let path_begin = path.chars().next();

                let url_path = if path_begin == Some('~') {
                    format!("/{}", path)
                } else if path_begin != Some('/') {
                    format!("/./{}", path)
                } else {
                    path.to_string()
                };

                format!("ssh://{}{}", target, url_path)
            } else {
                return Err(gettext("Incomplete URL or borg syntax"));
            }
        } else {
            input
        };

        match glib::Uri::parse(&url, glib::UriFlags::NONE) {
            Ok(uri) => {
                if uri.path().is_empty() {
                    return Err(gettext("The remote location must have a specified path."));
                }
            }
            Err(err) => {
                return Err(gettextf("Invalid remote location: “{}”", &[err.message()]));
            }
        }

        Ok(Self { url })
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }

    pub fn as_gio_file(&self) -> gio::File {
        gio::File::for_uri(&self.url)
    }

    pub fn is_borg_host(&self) -> bool {
        self.url.get(..6) == Some("ssh://")
    }
}
