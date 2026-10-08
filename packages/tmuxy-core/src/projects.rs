//! Named projects over a flat list of tmux sessions.
//!
//! tmux has no place to hang "these sessions are the IHP PDK work". A project
//! is that grouping: a name and the session names that belong to it, stored in
//! `~/.config/tmuxy/projects.json`. It does not create, link, or hide sessions.
//! The switcher reads the file and draws the ones that exist on the attached
//! server under their project.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One project. Session names are tmux session names, not ids, because that is
/// what `switch-client -t` and the switcher both key on. A session belongs to
/// at most one project; assigning it again moves it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    #[serde(default)]
    pub sessions: Vec<String>,
}

pub fn projects_path() -> PathBuf {
    crate::session::config_dir().join("projects.json")
}

/// Projects for the switcher. A missing file is an empty list. A file that
/// exists but cannot be parsed is also empty here: the menu must still open.
/// Mutations use [`load`] so they refuse to overwrite a corrupt file.
pub fn read_projects() -> Vec<Project> {
    load_from(&projects_path()).unwrap_or_default()
}

pub fn list() -> Result<Vec<Project>, String> {
    load_from(&projects_path())
}

pub fn create(name: &str) -> Result<(), String> {
    create_at(&projects_path(), name)
}

pub fn remove(name: &str) -> Result<(), String> {
    remove_at(&projects_path(), name)
}

/// Put `session` in `project`, taking it out of whichever project had it.
pub fn assign(project: &str, session: &str) -> Result<(), String> {
    assign_at(&projects_path(), project, session)
}

pub fn unassign(session: &str) -> Result<(), String> {
    unassign_at(&projects_path(), session)
}

fn create_at(path: &Path, name: &str) -> Result<(), String> {
    let name = require_name(name, "project")?;
    let mut projects = load_from(path)?;
    if projects.iter().any(|p| p.name == name) {
        return Err(format!("project '{name}' already exists"));
    }
    projects.push(Project {
        name,
        sessions: Vec::new(),
    });
    write_to(path, &projects)
}

fn remove_at(path: &Path, name: &str) -> Result<(), String> {
    let name = require_name(name, "project")?;
    let mut projects = load_from(path)?;
    let before = projects.len();
    projects.retain(|p| p.name != name);
    if projects.len() == before {
        return Err(format!("no project named '{name}'"));
    }
    write_to(path, &projects)
}

fn assign_at(path: &Path, project: &str, session: &str) -> Result<(), String> {
    let project = require_name(project, "project")?;
    let session = require_name(session, "session")?;
    let mut projects = load_from(path)?;
    let Some(index) = projects.iter().position(|p| p.name == project) else {
        return Err(format!("no project named '{project}'"));
    };
    for existing in &mut projects {
        existing.sessions.retain(|s| s != &session);
    }
    projects[index].sessions.push(session);
    write_to(path, &projects)
}

fn unassign_at(path: &Path, session: &str) -> Result<(), String> {
    let session = require_name(session, "session")?;
    let mut projects = load_from(path)?;
    let mut found = false;
    for project in &mut projects {
        let before = project.sessions.len();
        project.sessions.retain(|s| s != &session);
        found |= project.sessions.len() != before;
    }
    if !found {
        return Err(format!("session '{session}' is not in a project"));
    }
    write_to(path, &projects)
}

fn require_name(value: &str, what: &str) -> Result<String, String> {
    let name = value.trim();
    if name.is_empty() {
        return Err(format!("{what} name is empty"));
    }
    Ok(name.to_string())
}

fn load_from(path: &Path) -> Result<Vec<Project>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(Vec::new()),
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| format!("could not parse {}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("could not read {}: {e}", path.display())),
    }
}

fn write_to(path: &Path, projects: &[Project]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let body = serde_json::to_string_pretty(projects).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, format!("{body}\n")).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "tmuxy-projects-{}-{}-{label}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn missing_file_is_an_empty_list() {
        let path = scratch("missing");
        assert_eq!(load_from(&path).unwrap(), Vec::new());
    }

    #[test]
    fn assign_moves_a_session_out_of_its_previous_project() {
        let path = scratch("move");
        create_at(&path, "ihp").unwrap();
        create_at(&path, "docs").unwrap();
        assign_at(&path, "ihp", "pdk").unwrap();
        assign_at(&path, "docs", "pdk").unwrap();

        let projects = load_from(&path).unwrap();
        assert!(projects
            .iter()
            .find(|p| p.name == "ihp")
            .unwrap()
            .sessions
            .is_empty());
        assert_eq!(
            projects.iter().find(|p| p.name == "docs").unwrap().sessions,
            vec!["pdk".to_string()]
        );
        unassign_at(&path, "pdk").unwrap();
        assert!(load_from(&path)
            .unwrap()
            .iter()
            .all(|p| p.sessions.is_empty()));
        let _ = std::fs::remove_file(&path);
    }
}
