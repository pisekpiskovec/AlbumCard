use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

/// Separator used between names for `%{artists}`.
pub const ARTIST_SEPARATOR: &str = ", ";

const TOKENS: [&str; 8] = [
    "artists", "artist", "title", "album", "year", "aartist", "track", "disk",
];

/// Everything a pattern can refer to, for one track.
#[derive(Debug, Clone, Default)]
pub struct NameParts {
    pub artists: Vec<String>,
    pub album_artists: Vec<String>,
    pub title: String,
    pub album: String,
    pub year: Option<i32>,
    pub track: Option<u32>,
    pub disk: Option<u32>,
}

/// Checks a pattern without rendering it.
/// The error lists what is wrong in words the user can act on.
pub fn validate_pattern(pattern: &str) -> Result<(), String> {
    if pattern.trim().is_empty() {
        return Err("The pattern is empty".to_string());
    }

    let mut unknown = Vec::new();
    let mut rest = pattern;
    while let Some(start) = rest.find("%{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}") else {
            return Err("Unclosed %{\" in the pattern".to_string());
        };
        let name = &after[..end];
        if !TOKENS.contains(&name) {
            unknown.push(format!("%{{{name}}}"));
        }
        rest = &after[end + 1..];
    }
    if unknown.is_empty() {
        Ok(())
    } else {
        Err(format!("Unknown token: {}", unknown.join(", ")))
    }
}

/// Replaces characters that cannot appear in a file name, tidies whitespace and dots, and keeps the result within `max_bytes`.
fn sanitize(raw: &str, max_bytes: usize) -> String {
    let replaced: String = raw
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| if c == '/' { '-' } else { c })
        .map(|c| if c == ':' { '-' } else { c })
        .map(|c| if c == '?' { '_' } else { c })
        .map(|c| if c == '*' { '-' } else { c })
        .map(|c| if c == '"' { '-' } else { c })
        .map(|c| if c == '<' { '-' } else { c })
        .map(|c| if c == '>' { '-' } else { c })
        .map(|c| if c == '|' { '-' } else { c })
        .map(|c| if c == '\\' { '-' } else { c })
        .collect();
    let collapsed = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = collapsed.trim_matches(|c: char| c == '.' || c == ' ').to_string();
    while out.len() > max_bytes {
        out.pop();
    }
    out.trim_end().to_string()
}

/// Renders `pattern` for one track into a file name stem (no extension).
/// Returns `None` when nothing usable is left.
pub fn render(pattern: &str, parts: &NameParts) -> Option<String> {
    let mut out = String::new();
    let mut rest = pattern;
    while let Some(start) = rest.find("%{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let name = &after[..end];
        match name {
            "artists" => out.push_str(&parts.artists.join(ARTIST_SEPARATOR)),
            "artist" => out.push_str(parts.artists.first().map(String::as_str).unwrap_or("")),
            "title" => out.push_str(&parts.title),
            "album" => out.push_str(&parts.album),
            "year" => {
                if let Some(y) = parts.year {
                    out.push_str(&y.to_string());
                }
            }
            "aartist" => out.push_str(parts.album_artists.first().map(String::as_str).unwrap_or("")),
            "track" => {
                if let Some(t) = parts.track {
                    out.push_str(&format!("{t:02}"));
                }
            }
            "disk" => {
                if let Some(d) = parts.disk {
                    out.push_str(&d.to_string());
                }
            }
            _ => {
                out.push_str("%{");
                out.push_str(name);
                out.push('}');
            }
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);

    // Leave room for the extension inside the usual 255-byte limit.
    let cleaned = sanitize(&out, 240);
    if cleaned.is_empty() { None } else { Some(cleaned) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedRename {
    pub from: PathBuf,
    pub to: PathBuf,
}

/// Builds the list of renames for `{current path, new stem}` pairs.
/// The directory and extension are kept. Files whose name would not change are left out.
/// Nothing is touched on disk; any problem is returned instead:
/// two files that would get the same name, or a name already taken by a file that is not part of this rename.
pub fn plan(items: &[(PathBuf, String)]) -> Result<Vec<PlannedRename>, Vec<String>> {
    let mut errors = Vec::new();
    let mut planned = Vec::new();
    let sources: HashSet<&Path> = items.iter().map(|(p, _)| p.as_path()).collect();
    let mut taken: HashMap<PathBuf, &Path> = HashMap::new();

    for (from, stem) in items {
        let mut name = stem.clone();
        if let Some(ext) = from.extension().and_then(|e| e.to_str()) {
            name.push('.');
            name.push_str(ext);
        }
        let to = from.with_file_name(&name);

        if let Some(other) = taken.insert(to.clone(), from) {
            errors.push(format!(
                "\"{name}\" would be used by both {} and {}",
                other.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                from.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
            ));
            continue;
        }
        if &to == from {
            continue;
        }
        if to.exists() && !sources.contains(to.as_path()) {
            errors.push(format!("\"{name}\" already exists"));
            continue;
        }
        planned.push(PlannedRename { from: from.clone(), to });
    }

    if errors.is_empty() { Ok(planned) } else { Err(errors) }
}

fn rollback(staged: &[(PathBuf, PathBuf)]) {
    for (temp, original) in staged {
        let _ = std::fs::rename(temp, original);
    }
}

/// Carries out a plan in two steps (everything to a temporary name, then to the final name) so names can be swapped safely.
/// If something fails, what was already moved is moved back.
pub fn apply(plan: &[PlannedRename]) -> Result<(), String> {
    let mut staged: Vec<(PathBuf, PathBuf)> = Vec::new();
    for (i, step) in plan.iter().enumerate() {
        let temp = step
            .from
            .with_file_name(format!(".albumcard-rename-{}-{i}.tmp", std::process::id()));
        if let Err(e) = std::fs::rename(&step.from, &temp) {
            rollback(&staged);
            return Err(format!("{}: {e}", step.from.display()));
        }
        staged.push((temp, step.from.clone()));
    }

    let mut finished: Vec<usize> = Vec::new();
    for (i, step) in plan.iter().enumerate() {
        if let Err(e) = std::fs::rename(&staged[i].0, &step.to) {
            for &done in &finished {
                let _ = std::fs::rename(&plan[done].to, &staged[done].0);
            }
            rollback(&staged);
            return Err(format!("{}: {e}", step.to.display()));
        }
        finished.push(i);
    }

    Ok(())
}
