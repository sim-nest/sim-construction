//! Canonical, version-neutral paths for persistent construction project books.
//!
//! The layout deliberately has no backend or schema-version component:
//!
//! ```text
//! projects/<ProjectId>/facts/<seq>
//! projects/<ProjectId>/baselines/<id>
//! projects/<ProjectId>/policies/<id>
//! projects/<ProjectId>/projections/<name>/<as-of>
//! ```
//!
//! Facts, baselines, and policies are authoritative ordinary expressions.
//! Projections are disposable derived caches. Every dynamic component passes
//! through [`TablePath`] rather than being concatenated into an unchecked host
//! path.

use sim_table_core::{TablePath, TablePathError};

use crate::{BaselineId, ProjectId};

/// Canonical Table/Dir path layout for one construction project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectBookLayout {
    project: ProjectId,
}

impl ProjectBookLayout {
    /// Binds the canonical layout to one project.
    pub fn new(project: ProjectId) -> Result<Self, TablePathError> {
        let layout = Self { project };
        layout.project_root()?;
        Ok(layout)
    }

    /// Returns the project bound to this layout.
    #[must_use]
    pub fn project(&self) -> &ProjectId {
        &self.project
    }

    /// Returns `projects/<ProjectId>`.
    pub fn project_root(&self) -> Result<TablePath, TablePathError> {
        TablePath::from_segments(["projects", self.project.as_str()])
    }

    /// Returns `projects/<ProjectId>/facts`.
    pub fn facts(&self) -> Result<TablePath, TablePathError> {
        self.path(["facts"])
    }

    /// Returns `projects/<ProjectId>/facts/<seq>`.
    pub fn fact(&self, sequence: u64) -> Result<TablePath, TablePathError> {
        self.path(["facts", &sequence.to_string()])
    }

    /// Returns `projects/<ProjectId>/baselines/<id>`.
    pub fn baseline(&self, id: &BaselineId) -> Result<TablePath, TablePathError> {
        self.path(["baselines", id.as_str()])
    }

    /// Returns `projects/<ProjectId>/policies/<id>`.
    pub fn policy(&self, id: &str) -> Result<TablePath, TablePathError> {
        self.path(["policies", id])
    }

    /// Returns `projects/<ProjectId>/projections/<name>/<as-of>`.
    pub fn projection(&self, name: &str, as_of: u64) -> Result<TablePath, TablePathError> {
        self.path(["projections", name, &as_of.to_string()])
    }

    fn path<const N: usize>(&self, suffix: [&str; N]) -> Result<TablePath, TablePathError> {
        let mut path = self.project_root()?;
        for segment in suffix {
            path.push(segment)?;
        }
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use sim_table_core::TablePathError;

    use super::ProjectBookLayout;
    use crate::{BaselineId, ProjectId};

    #[test]
    fn layout_is_version_neutral_and_canonical() {
        let layout = ProjectBookLayout::new(ProjectId::new("project-alpha").unwrap()).unwrap();
        assert_eq!(
            layout.fact(42).unwrap().join(),
            "projects/project-alpha/facts/42"
        );
        assert_eq!(
            layout
                .baseline(&BaselineId::new("scope-main").unwrap())
                .unwrap()
                .join(),
            "projects/project-alpha/baselines/scope-main"
        );
        assert_eq!(
            layout.policy("gate-main").unwrap().join(),
            "projects/project-alpha/policies/gate-main"
        );
        assert_eq!(
            layout.projection("snapshot", 42).unwrap().join(),
            "projects/project-alpha/projections/snapshot/42"
        );
    }

    #[test]
    fn dynamic_components_cannot_escape_the_table_path() {
        let layout = ProjectBookLayout::new(ProjectId::new("project-alpha").unwrap()).unwrap();
        assert!(matches!(
            layout.policy("../outside"),
            Err(TablePathError::IllegalSegment(_))
        ));
        assert!(matches!(
            layout.projection("nested/cache", 1),
            Err(TablePathError::IllegalSegment(_))
        ));
        assert!(matches!(
            layout.projection("..", 1),
            Err(TablePathError::IllegalSegment(_))
        ));
    }
}
