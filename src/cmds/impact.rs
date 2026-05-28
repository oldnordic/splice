//! Impact graph command handler.

use std::path::Path;

pub(crate) fn execute_impact_graph(
    db_path: &Path,
    symbol_id: &str,
    direction: &splice::cli::ReachabilityDirection,
    max_depth: Option<usize>,
) -> Result<splice::cli::CliSuccessPayload, splice::SpliceError> {
    use splice::graph::magellan_integration::{ImpactDotConfig, MagellanIntegration};

    let mut integration = MagellanIntegration::open(db_path)?;

    let config = ImpactDotConfig {
        show_symbol_kinds: true,
        max_depth,
        highlight_symbol: None,
    };

    let dot = integration.generate_impact_dot(symbol_id, direction, &config)?;

    // Output DOT to stdout
    println!("{}", dot);

    Ok(
        splice::cli::CliSuccessPayload::message_only("Impact graph generated".to_string())
            .already_emitted(),
    )
}
