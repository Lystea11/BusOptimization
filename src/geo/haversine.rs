use crate::data::structures::{Stop, Transfer, StopId, Time};

const EARTH_RADIUS_M: f32 = 6_371_000.0; // Earth radius in meters
const DEFAULT_WALKING_SPEED_MS: f32 = 1.4; // 5 km/h in m/s
const DEFAULT_BUS_SPEED_MS: f32 = 8.33; // 30 km/h in m/s

/// Calculate the Haversine distance between two points
pub fn calculate_distance(lat1: f32, lon1: f32, lat2: f32, lon2: f32) -> f32 {
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2) + 
            lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS_M * c
}

/// Calculate distance between two stops
pub fn distance_between_stops(stop1: &Stop, stop2: &Stop) -> f32 {
    calculate_distance(stop1.lat, stop1.lon, stop2.lat, stop2.lon)
}

/// Convert distance to walking time in seconds
pub fn distance_to_walk_time(distance_m: f32, walking_speed_ms: Option<f32>) -> Time {
    let speed = walking_speed_ms.unwrap_or(DEFAULT_WALKING_SPEED_MS);
    (distance_m / speed).ceil() as Time
}

/// Convert distance to bus travel time in seconds
pub fn distance_to_bus_time(distance_m: f32, bus_speed_ms: Option<f32>) -> Time {
    let speed = bus_speed_ms.unwrap_or(DEFAULT_BUS_SPEED_MS);
    (distance_m / speed).ceil() as Time
}

/// Generate walking transfers between stops within a given radius
pub fn generate_walking_transfers(
    stops: &[Stop], 
    max_walking_distance_m: f32,
    walking_speed_ms: Option<f32>
) -> Vec<Transfer> {
    let mut transfers = Vec::new();
    
    for (i, stop1) in stops.iter().enumerate() {
        for (j, stop2) in stops.iter().enumerate() {
            if i != j {
                let distance = distance_between_stops(stop1, stop2);
                
                if distance <= max_walking_distance_m {
                    let walk_time = distance_to_walk_time(distance, walking_speed_ms);
                    transfers.push(Transfer::new(
                        stop1.id,
                        stop2.id,
                        walk_time,
                        distance,
                    ));
                }
            }
        }
    }
    
    transfers
}

/// Configuration for geospatial calculations
#[derive(Debug, Clone)]
pub struct GeoConfig {
    pub walking_speed_ms: f32,      // Walking speed in m/s
    pub bus_speed_ms: f32,          // Bus speed in m/s  
    pub max_walking_distance_m: f32, // Maximum walking distance for transfers
}

impl Default for GeoConfig {
    fn default() -> Self {
        Self {
            walking_speed_ms: DEFAULT_WALKING_SPEED_MS,
            bus_speed_ms: DEFAULT_BUS_SPEED_MS,
            max_walking_distance_m: 300.0, // 300 meters
        }
    }
}

impl GeoConfig {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_walking_speed_kmh(mut self, speed_kmh: f32) -> Self {
        self.walking_speed_ms = speed_kmh / 3.6; // Convert km/h to m/s
        self
    }
    
    pub fn with_bus_speed_kmh(mut self, speed_kmh: f32) -> Self {
        self.bus_speed_ms = speed_kmh / 3.6; // Convert km/h to m/s
        self
    }
    
    pub fn with_max_walking_distance_m(mut self, distance_m: f32) -> Self {
        self.max_walking_distance_m = distance_m;
        self
    }
    
    /// Generate all walking transfers for the given stops
    pub fn generate_transfers(&self, stops: &[Stop]) -> Vec<Transfer> {
        generate_walking_transfers(stops, self.max_walking_distance_m, Some(self.walking_speed_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_distance() {
        // Straight-line distance between Tokyo Station and Shinjuku Station (approximately 6 km)
        let tokyo_lat = 35.6812;
        let tokyo_lon = 139.7671;
        let shinjuku_lat = 35.6896;
        let shinjuku_lon = 139.7006;

        let distance = calculate_distance(tokyo_lat, tokyo_lon, shinjuku_lat, shinjuku_lon);

        // Allow some margin for floating point precision
        assert!(distance > 5800.0 && distance < 6300.0, "Distance: {}", distance);
    }
    
    #[test]
    fn test_walking_time_calculation() {
        let distance_m = 300.0; // 300 meters
        let walk_time = distance_to_walk_time(distance_m, None);
        
        // At 1.4 m/s (5 km/h), 300m should take about 214 seconds
        assert!(walk_time > 200 && walk_time < 230, "Walk time: {}", walk_time);
    }
    
    #[test]
    fn test_bus_time_calculation() {
        let distance_m = 1000.0; // 1 km
        let bus_time = distance_to_bus_time(distance_m, None);
        
        // At 8.33 m/s (30 km/h), 1000m should take about 120 seconds
        assert!(bus_time > 110 && bus_time < 130, "Bus time: {}", bus_time);
    }
}