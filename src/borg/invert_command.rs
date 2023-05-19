///! Extract information from previous borg run command lines
use crate::config;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Clone, Debug)]
enum CreateTerm {
    OptExclude,
    OptExcludeFrom,
    OptExcludeIfPresent,
    OptExcludeNodump,
    OptExcludeCaches,
    UnknownOption,
    Value,
    Target,
}

impl CreateTerm {
    pub fn parse(s: String) -> Vec<(Self, String)> {
        if s == "--exclude-caches" {
            return vec![(Self::OptExcludeCaches, s)];
        }

        if s == "--exclude-nodump" {
            return vec![(Self::OptExcludeNodump, s)];
        }

        if s.starts_with("--exclude") || s.starts_with("-e") {
            let term = if s == "--exclude-from" || s.starts_with("--exclude-from=") {
                Self::OptExcludeFrom
            } else if s == "--exclude-if-present" || s.starts_with("--exclude-if-present=") {
                Self::OptExcludeIfPresent
            } else if s == "--exclude" || s.starts_with("--exclude=") || s.starts_with("-e") {
                Self::OptExclude
            } else {
                Self::UnknownOption
            };

            return if let Some((option, value)) = s.split_once('=') {
                vec![(term, option.to_string()), (Self::Value, value.to_string())]
            } else {
                vec![(term, s.clone())]
            };
        }

        if s.starts_with('-') {
            vec![(Self::UnknownOption, s)]
        } else if s.contains("::") {
            vec![(Self::Target, s)]
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

    let mut exclude_rules = BTreeSet::new();
    let mut options = ast_options.into_iter();

    while let Some(option) = options.next() {
        match &option.0 {
            CreateTerm::OptExclude => {
                if let Some((CreateTerm::Value, value)) = options.next() {
                    // TODO: why what?
                    if !value.ends_with(&format!(".var/app/{}/data/flatpak/", crate::APP_ID)) {
                        if let Some(pattern) = config::Pattern::from_borg(value) {
                            exclude_rules.insert(config::exclude::Rule::Pattern(pattern));
                        }
                    }
                }
            }
            CreateTerm::OptExcludeCaches => {
                exclude_rules.insert(config::exclude::Rule::CacheDirTag);
            }
            _ => {}
        }
    }

    let mut exclude = BTreeSet::new();

    // Transform patterns in to presets if it matches
    for predefined in config::exclude::Predefined::VALUES {
        let predefined_rules = BTreeSet::from_iter(predefined.rules().iter().cloned());

        if predefined_rules.is_subset(&exclude_rules) {
            for rule in predefined.rules() {
                exclude_rules.remove(rule);
            }
            exclude.insert(config::Exclude::from_predefined(predefined));
        }
    }

    // Add remaining patterns to exclude
    exclude.append(&mut BTreeSet::from_iter(
        exclude_rules
            .into_iter()
            .filter_map(|x| {
                if let config::exclude::Rule::Pattern(p) = x {
                    Some(p)
                } else {
                    None
                }
            })
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
                .rules()
                .iter()
                .filter_map(|x| {
                    if let config::exclude::Rule::Pattern(p) = x {
                        Some(p)
                    } else {
                        None
                    }
                })
                .map(|x| format!("--exclude={}", x.borg_pattern().into_string().unwrap()))
                .collect(),
        );

        cmd.append(&mut vec![
            "--exclude-caches".into(),
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
