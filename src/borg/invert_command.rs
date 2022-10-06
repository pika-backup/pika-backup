///! Extract information from previous borg run command lines
use crate::config;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Clone, Debug)]
enum CreateTerm {
    OptExclude,
    UnknownOption,
    Value,
    Target,
}

impl CreateTerm {
    pub fn parse(s: String) -> Vec<(Self, String)> {
        if s.contains("::") {
            return vec![(Self::Target, s)];
        }

        if s.starts_with("--exclude") || s.starts_with("-e") {
            let mut result = vec![(Self::OptExclude, s.clone())];

            if let Some((option, value)) = s.split_once('=') {
                result = vec![
                    (Self::OptExclude, option.to_string()),
                    (Self::Value, value.to_string()),
                ];
            }

            return result;
        }

        if s.starts_with('-') {
            vec![(Self::UnknownOption, s)]
        } else {
            vec![(Self::Value, s)]
        }
    }
}

fn ast(cmd: Vec<String>) -> Vec<(CreateTerm, String)> {
    cmd.into_iter().flat_map(CreateTerm::parse).collect()
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Parsed {
    pub exclude: BTreeSet<config::Exclude<{ config::ABSOLUTE }>>,
    pub include: BTreeSet<PathBuf>,
}

/// Currently `String` because borg returns string
pub fn parse(cmd: Vec<String>) -> Parsed {
    let ast = ast(cmd);

    let mut ast_split = ast.splitn(2, |x| matches!(x.0, CreateTerm::Target));

    let ast_options = ast_split.next().map(|x| x.to_vec()).unwrap_or_default();
    let ast_include = ast_split.next().map(|x| x.to_vec()).unwrap_or_default();

    let mut exclude_patterns = BTreeSet::new();
    let mut options = ast_options.into_iter();

    while let Some(option) = options.next() {
        if matches!(option.0, CreateTerm::OptExclude) {
            if let Some((CreateTerm::Value, value)) = options.next() {
                // TODO: why what?
                if !value.ends_with(&format!(".var/app/{}/data/flatpak/", crate::APP_ID)) {
                    if let Some(pattern) = config::Pattern::from_borg(value) {
                        exclude_patterns.insert(pattern);
                    }
                }
            }
        }
    }

    let mut exclude = BTreeSet::new();

    // Transform patterns in to presets if it matches
    for predefined in config::exclude::Predefined::VALUES {
        let predefined_patterns = BTreeSet::from_iter(predefined.patterns().iter().cloned());

        if predefined_patterns.is_subset(&exclude_patterns) {
            for pattern in predefined.patterns() {
                exclude_patterns.remove(pattern);
            }
            exclude.insert(config::Exclude::from_predefined(predefined));
        }
    }

    // Add remaining patterns to exclude
    exclude.append(&mut BTreeSet::from_iter(
        exclude_patterns
            .into_iter()
            .map(config::Exclude::from_pattern),
    ));

    let include = ast_include
        .into_iter()
        .map(|x| PathBuf::from(x.1))
        .map(|x| {
            x.strip_prefix(glib::home_dir())
                .map(|x| x.to_path_buf())
                .unwrap_or(x)
        })
        .collect();

    Parsed { exclude, include }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic() {
        let cmd = [
            "/app/bin/borg",
            "create",
            "--rsh",
            "ssh -o BatchMode=yes",
            "--progress",
            "--json",
            "--compression=zstd",
            "--log-json",
            "--exclude=pp:/home/xuser/.cache",
            "--",
            "ssh://example.org/./repo::prefix-53070a25",
            "/home/xuser/Music",
        ]
        .iter()
        .map(|x| x.to_string())
        .collect();

        assert_eq!(
            parse(cmd),
            Parsed {
                exclude: [config::Exclude::from_pattern(config::Pattern::PathPrefix(
                    PathBuf::from("/home/xuser/.cache")
                )),]
                .into(),
                include: [PathBuf::from("/home/xuser/Music")].into(),
            }
        );
    }

    #[test]
    fn presets() {
        let mut cmd = vec!["/app/bin/borg".into(), "create".into()];

        cmd.append(
            &mut config::exclude::Predefined::Caches
                .patterns()
                .iter()
                .map(|x| format!("--exclude={}", x.borg_pattern().into_string().unwrap()))
                .collect(),
        );

        cmd.append(&mut vec![
            "ssh://example.org/./repo::prefix-53070a25".into(),
            "/home/xuser/Music".into(),
        ]);

        assert_eq!(
            parse(cmd),
            Parsed {
                exclude: [config::Exclude::from_predefined(
                    config::exclude::Predefined::Caches
                ),]
                .into(),
                include: [PathBuf::from("/home/xuser/Music")].into(),
            }
        );
    }
}
