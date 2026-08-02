//! Reference-only pointers from office rows to construction fact evidence.

use sim_lib_construction_project::ProjectFact;
use sim_lib_doc_core::ExternalRef;

use crate::EvidenceBridgeError;

const CONSTRUCTION_FACT_BACKEND: &str = "construction/project-fact";

pub(crate) fn construction_fact_ref(fact: &ProjectFact, evidence_index: usize) -> ExternalRef {
    ExternalRef::new(
        CONSTRUCTION_FACT_BACKEND,
        format!(
            "projects/{}/controls/{}/facts/{}/evidence/{evidence_index}",
            fact.project, fact.subject, fact.seq
        ),
        None,
        None,
    )
}

pub(crate) struct FactPointer<'a> {
    pub(crate) project: &'a str,
    pub(crate) control: &'a str,
    pub(crate) sequence: u64,
    pub(crate) evidence_index: usize,
}

impl<'a> FactPointer<'a> {
    pub(crate) fn parse(reference: &'a ExternalRef) -> Result<Option<Self>, EvidenceBridgeError> {
        if reference.backend != CONSTRUCTION_FACT_BACKEND {
            return Ok(None);
        }
        let parts = reference.external_id.split('/').collect::<Vec<_>>();
        if parts.len() != 8
            || parts[0] != "projects"
            || parts[2] != "controls"
            || parts[4] != "facts"
            || parts[6] != "evidence"
        {
            return Err(EvidenceBridgeError::InvalidProjection(format!(
                "malformed fact reference {}",
                reference.external_id
            )));
        }
        let sequence = parts[5].parse::<u64>().map_err(|_| {
            EvidenceBridgeError::InvalidProjection(format!(
                "invalid fact sequence in {}",
                reference.external_id
            ))
        })?;
        let evidence_index = parts[7].parse::<usize>().map_err(|_| {
            EvidenceBridgeError::InvalidProjection(format!(
                "invalid evidence slot in {}",
                reference.external_id
            ))
        })?;
        Ok(Some(Self {
            project: parts[1],
            control: parts[3],
            sequence,
            evidence_index,
        }))
    }
}
