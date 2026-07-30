use sim_lib_doc_core::LinkRole;

use crate::{
    commercial_support_relation, design_source_relation, field_issue_relation,
    published_deliverable_relation, schedule_basis_relation,
};

#[test]
fn precise_construction_relations_map_to_existing_office_roles() {
    let mappings = [
        (design_source_relation(), LinkRole::SourceDocument),
        (schedule_basis_relation(), LinkRole::ScheduleReference),
        (field_issue_relation(), LinkRole::ProjectIssue),
        (commercial_support_relation(), LinkRole::AccountingSupport),
        (published_deliverable_relation(), LinkRole::PublishedTo),
    ];

    for (relation, expected) in mappings {
        assert_eq!(relation.office_role(), expected);
        assert_eq!(
            relation.fact_kind().namespace.as_deref(),
            Some("construction-evidence")
        );
    }
}
