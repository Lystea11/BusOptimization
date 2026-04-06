pub mod backward;
pub mod journey;
pub mod private_bus;
pub mod geojson;

pub use backward::BackwardRaptor;
pub use journey::{parse_time, JourneySummary};
pub use private_bus::PrivateBusPlanner;

/// Core RAPTOR algorithm interface
pub trait RaptorAlgorithm {
    type Journey;

    /// Find the fastest journey between two stops
    fn find_journey(
        &self, 
        origin: u32, 
        destination: u32, 
        departure_time: u32
    ) -> Option<Self::Journey>;
}

/// A trait for journey types returned by RAPTOR algorithms
pub trait RaptorJourney {
    /// Get the total travel time
    fn total_time(&self) -> u32;

    /// Get the number of transfers
    fn num_transfers(&self) -> usize;

    /// Get a summary of the journey
    fn summary(&self) -> String;
}
