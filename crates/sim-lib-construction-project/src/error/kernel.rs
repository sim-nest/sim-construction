//! Kernel error adaptation for construction domain failures.

use super::ConstructionProjectError;

impl From<ConstructionProjectError> for sim_kernel::Error {
    fn from(error: ConstructionProjectError) -> Self {
        Self::Eval(error.to_string())
    }
}
