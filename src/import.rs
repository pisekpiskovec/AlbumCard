use std::{cmp::Ordering, iter::Peekable, path::{Path, PathBuf}, str::Chars};

const AUDIO_EXTENSIONS: [&str; 1] = ["mp3"];
/// Same as the "Load Album Art" dialog accepts.
const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

/// What a drop contains, sorted out and ready to apply.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DropPlan {
    /// Audio files in the order they should be added.
    pub audio: Vec<PathBuf>,
    /// The image to use as album art (`cover.*` winds, else the first by name).
    pub cover: Option<PathBuf>,
    /// Files that were neither supported audio nor a supported image.
    pub ignored: usize,
}

fn has_extension(path: &Path, list: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| list.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

fn is_cover_name(path: &Path) -> bool {
    path.file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("cover"))
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}

fn take_number(it: &mut Peekable<Chars>) -> String {
    let mut digits = String::new();
    while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
        digits.push(c);
        it.next();
    }
    digits.trim_start_matches('0').to_string()
}

pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let (nx, ny) = (take_number(&mut a), take_number(&mut b));
                let order = nx.len().cmp(&ny.len()).then_with(|| nx.cmp(&ny));
                if order != Ordering::Equal {
                    return order;
                }
            },
            (Some(x), Some(y)) => {
                let order = x.to_lowercase().cmp(y.to_lowercase());
                if order != Ordering::Equal {
                    return order;
                }
                a.next();
                b.next();
            }
        }
    }
}

fn path_cmp(a: &Path, b: &Path) -> Ordering {
    let name = |p: &Path| p.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
    let dir = |p: &Path| p.parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default();
    natural_cmp(&dir(a), &dir(b)).then_with(|| natural_cmp(&name(a), &name(b)))
}

fn classify(path: PathBuf, audio: &mut Vec<PathBuf>, images: &mut Vec<PathBuf>, ignored: &mut usize) {
    if has_extension(&path, &AUDIO_EXTENSIONS) {
        audio.push(path);
    } else if has_extension(&path, &IMAGE_EXTENSIONS) {
        images.push(path);
    } else {
        *ignored += 1;
    }
}

/// Sorts what was dropped (files and/or folders) into audio and cover.
/// Folders are read one level deep; hidden files and sub-folders are skipped.
/// Never fails: anything unreadible is simply not included.
pub fn plan_drop(paths: &[PathBuf]) -> DropPlan {
    let mut audio = Vec::new();
    let mut images = Vec::new();
    let mut ignored = 0;

    for path in paths {
        if path.is_dir() {
            let Ok(entries) = std::fs::read_dir(path) else { continue };
            for entry in entries.flatten() {
                let inner = entry.path();
                if inner.is_file() && !is_hidden(&inner) {
                    classify(inner, &mut audio, &mut images, &mut ignored);
                }
            }
        } else if path.is_file() {
            classify(path.clone(), &mut audio, &mut images, &mut ignored);
        }
    }

    audio.sort_by(|a, b| path_cmp(a, b));
    audio.dedup();
    images.sort_by(|a, b| path_cmp(a, b));

    let cover = images.iter().find(|p| is_cover_name(p)).or_else(|| images.first()).cloned();

    DropPlan { audio, cover, ignored }
}
