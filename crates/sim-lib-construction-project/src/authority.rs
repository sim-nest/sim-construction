//! Library-owned construction project-control capability names.

use sim_kernel::CapabilityName;

/// Capability required to read construction project-control records.
pub const CONSTRUCTION_PROJECT_READ_CAPABILITY: &str = "construction.project.read";
/// Capability required to write construction project-control records.
pub const CONSTRUCTION_PROJECT_WRITE_CAPABILITY: &str = "construction.project.write";
/// Capability required to accept project-control records.
pub const CONSTRUCTION_PROJECT_ACCEPT_CAPABILITY: &str = "construction.project.accept";
/// Capability required to record construction exceptions.
pub const CONSTRUCTION_EXCEPTION_CAPABILITY: &str = "construction.exception";
/// Capability required to publish construction reference candidates.
pub const CONSTRUCTION_REFERENCE_PUBLISH_CAPABILITY: &str = "construction.reference.publish";

/// Builds the project-read capability name.
#[must_use]
pub fn construction_project_read_capability() -> CapabilityName {
    CapabilityName::new(CONSTRUCTION_PROJECT_READ_CAPABILITY)
}

/// Builds the project-write capability name.
#[must_use]
pub fn construction_project_write_capability() -> CapabilityName {
    CapabilityName::new(CONSTRUCTION_PROJECT_WRITE_CAPABILITY)
}

/// Builds the project-accept capability name.
#[must_use]
pub fn construction_project_accept_capability() -> CapabilityName {
    CapabilityName::new(CONSTRUCTION_PROJECT_ACCEPT_CAPABILITY)
}

/// Builds the construction-exception capability name.
#[must_use]
pub fn construction_exception_capability() -> CapabilityName {
    CapabilityName::new(CONSTRUCTION_EXCEPTION_CAPABILITY)
}

/// Builds the reference-publish capability name.
#[must_use]
pub fn construction_reference_publish_capability() -> CapabilityName {
    CapabilityName::new(CONSTRUCTION_REFERENCE_PUBLISH_CAPABILITY)
}
