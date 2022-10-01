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
    pub exclude: BTreeSet<config::Exclude>,
    pub include: BTreeSet<PathBuf>,
}

pub fn parse(cmd: Vec<String>) -> Parsed {
    let ast = ast(cmd);

    let mut ast_split = ast.splitn(2, |x| matches!(x.0, CreateTerm::Target));

    let ast_options = ast_split.next().map(|x| x.to_vec()).unwrap_or_default();
    let ast_include = ast_split.next().map(|x| x.to_vec()).unwrap_or_default();

    let mut exclude = BTreeSet::new();
    let mut options = ast_options.into_iter();

    while let Some(option) = options.next() {
        if matches!(option.0, CreateTerm::OptExclude) {
            if let Some((CreateTerm::Value, value)) = options.next() {
                if !value.ends_with(&format!(".var/app/{}/data/flatpak/", crate::APP_ID)) {
                    if let Some(pattern) = config::Pattern::from_borg(value) {
                        exclude.insert(config::Exclude::from_pattern(pattern));
                    }
                }
            }
        }
    }

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

#[test]
fn test() {
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
        "--exclude=pp:/home/xuser/.mnt/borg",
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
