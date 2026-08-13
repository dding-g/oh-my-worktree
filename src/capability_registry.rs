//! Checked inventory of the user-facing CLI capabilities and their TUI parity.
//!
//! This is deliberately data, rather than a duplicate command implementation.
//! It makes parity debt visible in code review and gives every completed
//! capability a scenario identifier that can be traced to tests.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParityStatus {
    Full,
    Partial,
    Missing,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Capability {
    pub id: &'static str,
    pub cli_contract: &'static str,
    pub tui_surface: &'static str,
    pub status: ParityStatus,
    pub scenario: Option<&'static str>,
    pub delivery_horizon: Option<&'static str>,
    pub gap_reason: Option<&'static str>,
}

pub(crate) const CAPABILITIES: &[Capability] = &[
    Capability {
        id: "C01",
        cli_contract: "Open/list repository worktrees",
        tui_surface: "List view and refresh",
        status: ParityStatus::Full,
        scenario: Some("UC_REGULAR_START"),
        delivery_horizon: None,
        gap_reason: None,
    },
    Capability {
        id: "C02",
        cli_contract: "Clone .bare workspace",
        tui_surface: "Global home clone flow",
        status: ParityStatus::Missing,
        scenario: None,
        delivery_horizon: Some("H5"),
        gap_reason: Some("TUI requires an existing repository at launch"),
    },
    Capability {
        id: "C03",
        cli_contract: "Show init conversion guide",
        tui_surface: "Global home init guide",
        status: ParityStatus::Missing,
        scenario: None,
        delivery_horizon: Some("H5"),
        gap_reason: Some("No global TUI home"),
    },
    Capability {
        id: "C04",
        cli_contract: "Install shell setup",
        tui_surface: "Global home setup preview and execute",
        status: ParityStatus::Partial,
        scenario: None,
        delivery_horizon: Some("H5"),
        gap_reason: Some("TUI can use shell handoff but cannot install setup"),
    },
    Capability {
        id: "C05",
        cli_contract: "Create with base, path, and tmux choice",
        tui_surface: "Progressive add flow",
        status: ParityStatus::Full,
        scenario: Some("UC_ADD_WORKTREE"),
        delivery_horizon: None,
        gap_reason: None,
    },
    Capability {
        id: "C06",
        cli_contract: "Delete worktree with force and branch choice",
        tui_surface: "Batch delete confirmation",
        status: ParityStatus::Full,
        scenario: Some("UC_DELETE_WORKTREE"),
        delivery_horizon: None,
        gap_reason: None,
    },
    Capability {
        id: "C07",
        cli_contract: "Prune completed-PR worktrees",
        tui_surface: "Cleanup preview and confirmation",
        status: ParityStatus::Full,
        scenario: Some("UC_PRUNE_COMPLETED"),
        delivery_horizon: None,
        gap_reason: None,
    },
    Capability {
        id: "C08",
        cli_contract: "Check PR status",
        tui_surface: "PR detail/query modal",
        status: ParityStatus::Full,
        scenario: Some("UC_PR_STATUS"),
        delivery_horizon: None,
        gap_reason: None,
    },
    Capability {
        id: "C09",
        cli_contract: "Browse commit tree with limit",
        tui_surface: "Commit tree screen with paging",
        status: ParityStatus::Full,
        scenario: Some("UC_COMMIT_TREE"),
        delivery_horizon: None,
        gap_reason: None,
    },
    Capability {
        id: "C10",
        cli_contract: "Search path/name/branch/status/PR",
        tui_surface: "Slash filter",
        status: ParityStatus::Full,
        scenario: Some("UC_SEARCH_WORKTREE"),
        delivery_horizon: None,
        gap_reason: None,
    },
    Capability {
        id: "C11",
        cli_contract: "Browse CLI help",
        tui_surface: "Command help browser",
        status: ParityStatus::Partial,
        scenario: Some("UC_HELP"),
        delivery_horizon: Some("H5"),
        gap_reason: Some("TUI help is keybinding-only"),
    },
    Capability {
        id: "C12",
        cli_contract: "Show version/about",
        tui_surface: "About modal",
        status: ParityStatus::Missing,
        scenario: None,
        delivery_horizon: Some("H5"),
        gap_reason: Some("No about modal"),
    },
];

pub(crate) fn invariant_holds() -> bool {
    CAPABILITIES.len() == 12
        && CAPABILITIES.iter().enumerate().all(|(index, capability)| {
            let unique_id = CAPABILITIES[..index]
                .iter()
                .all(|prior| prior.id != capability.id);
            let complete = capability.scenario.is_some()
                && capability.delivery_horizon.is_none()
                && capability.gap_reason.is_none();
            let incomplete =
                capability.delivery_horizon.is_some() && capability.gap_reason.is_some();

            unique_id
                && !capability.cli_contract.is_empty()
                && !capability.tui_surface.is_empty()
                && match capability.status {
                    ParityStatus::Full => complete,
                    ParityStatus::Partial | ParityStatus::Missing => incomplete,
                }
        })
}

#[cfg(test)]
pub(crate) fn release_blockers() -> impl Iterator<Item = &'static Capability> {
    CAPABILITIES
        .iter()
        .filter(|capability| capability.status != ParityStatus::Full)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{invariant_holds, release_blockers, ParityStatus, CAPABILITIES};

    #[test]
    fn registry_invariant_holds() {
        assert!(invariant_holds());
    }

    #[test]
    fn every_ga_capability_has_a_unique_cli_and_tui_contract() {
        let mut ids = HashSet::new();
        for capability in CAPABILITIES {
            assert!(
                ids.insert(capability.id),
                "duplicate capability ID: {}",
                capability.id
            );
            assert!(!capability.cli_contract.is_empty());
            assert!(!capability.tui_surface.is_empty());
        }
        assert_eq!(CAPABILITIES.len(), 12);
    }

    #[test]
    fn full_capabilities_require_a_traceable_scenario() {
        for capability in CAPABILITIES
            .iter()
            .filter(|capability| capability.status == ParityStatus::Full)
        {
            assert!(
                capability.scenario.is_some(),
                "{} has no scenario",
                capability.id
            );
            assert!(capability.delivery_horizon.is_none());
            assert!(capability.gap_reason.is_none());
        }
    }

    #[test]
    fn incomplete_capabilities_remain_explicit_release_blockers() {
        for capability in release_blockers() {
            assert!(
                capability.delivery_horizon.is_some(),
                "{} has no horizon",
                capability.id
            );
            assert!(
                capability.gap_reason.is_some(),
                "{} has no gap reason",
                capability.id
            );
        }
    }
}
