pub mod data;
pub mod geo;
pub mod raptor;
pub mod cli;

pub use data::structures::{Stop, Route, Transfer, Network};
pub use raptor::backward::BackwardRaptor;
pub use geo::haversine::calculate_distance;

pub type StopId = u32;
pub type RouteId = u32;
pub type Time = u32; // Seconds since midnight