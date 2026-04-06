use crate::data::structures::{Stop, Route, Transfer, Network, StopId, RouteId};
use crate::geo::haversine::GeoConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct StopCsv {
    pub id: StopId,
    pub name: String,
    pub lat: f32,
    pub lon: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RouteCsv {
    pub id: RouteId,
    pub name: String,
    pub stops: String,        // Comma-separated stop IDs
    pub travel_times: String, // Comma-separated travel times in seconds
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TransferCsv {
    pub from_stop: StopId,
    pub to_stop: StopId,
    pub walk_time: u32,
    pub distance: f32,
}

/// Load stops from CSV file
pub fn load_stops_csv<P: AsRef<Path>>(path: P) -> Result<Vec<Stop>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut csv_reader = csv::Reader::from_reader(reader);
    
    let mut stops = Vec::new();
    for result in csv_reader.deserialize() {
        let record: StopCsv = result?;
        stops.push(Stop::new(record.id, record.name, record.lat, record.lon));
    }
    
    Ok(stops)
}

/// Load routes from CSV file
pub fn load_routes_csv<P: AsRef<Path>>(path: P) -> Result<Vec<Route>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut csv_reader = csv::Reader::from_reader(reader);
    
    let mut routes = Vec::new();
    for result in csv_reader.deserialize() {
        let record: RouteCsv = result?;
        
        // Parse stop IDs
        let stop_ids: Result<Vec<StopId>, _> = record.stops
            .split(',')
            .map(|s| s.trim().parse::<StopId>())
            .collect();
        let stop_ids = stop_ids.map_err(|e| format!("Failed to parse stop IDs: {}", e))?;
        
        // Parse travel times
        let travel_times: Result<Vec<u32>, _> = record.travel_times
            .split(',')
            .map(|s| s.trim().parse::<u32>())
            .collect();
        let travel_times = travel_times.map_err(|e| format!("Failed to parse travel times: {}", e))?;
        
        routes.push(Route::new(record.id, record.name, stop_ids, travel_times));
    }
    
    Ok(routes)
}

/// Load transfers from CSV file
pub fn load_transfers_csv<P: AsRef<Path>>(path: P) -> Result<Vec<Transfer>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut csv_reader = csv::Reader::from_reader(reader);
    
    let mut transfers = Vec::new();
    for result in csv_reader.deserialize() {
        let record: TransferCsv = result?;
        transfers.push(Transfer::new(
            record.from_stop,
            record.to_stop,
            record.walk_time,
            record.distance,
        ));
    }
    
    Ok(transfers)
}

/// Save stops to CSV file
pub fn save_stops_csv<P: AsRef<Path>>(stops: &[Stop], path: P) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let mut csv_writer = csv::Writer::from_writer(writer);
    
    for stop in stops {
        let record = StopCsv {
            id: stop.id,
            name: stop.name.clone(),
            lat: stop.lat,
            lon: stop.lon,
        };
        csv_writer.serialize(record)?;
    }
    
    csv_writer.flush()?;
    Ok(())
}

/// Save routes to CSV file
pub fn save_routes_csv<P: AsRef<Path>>(routes: &[Route], path: P) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let mut csv_writer = csv::Writer::from_writer(writer);
    
    for route in routes {
        let stops_str = route.stops.iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        
        let times_str = route.travel_times.iter()
            .map(|time| time.to_string())
            .collect::<Vec<_>>()
            .join(",");
        
        let record = RouteCsv {
            id: route.id,
            name: route.name.clone(),
            stops: stops_str,
            travel_times: times_str,
        };
        csv_writer.serialize(record)?;
    }
    
    csv_writer.flush()?;
    Ok(())
}

/// Load stops from JSON file
pub fn load_stops_json<P: AsRef<Path>>(path: P) -> Result<Vec<Stop>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let stops: Vec<Stop> = serde_json::from_reader(reader)?;
    Ok(stops)
}

/// Load routes from JSON file
pub fn load_routes_json<P: AsRef<Path>>(path: P) -> Result<Vec<Route>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let routes: Vec<Route> = serde_json::from_reader(reader)?;
    Ok(routes)
}

/// Save network to JSON file
pub fn save_network_json<P: AsRef<Path>>(network: &Network, path: P) -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Serialize)]
    struct NetworkJson<'a> {
        stops: &'a Vec<Stop>,
        routes: &'a Vec<Route>,
        transfers: &'a Vec<Transfer>,
    }
    
    let network_json = NetworkJson {
        stops: &network.stops,
        routes: &network.routes,
        transfers: &network.transfers,
    };
    
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &network_json)?;
    Ok(())
}

/// Load complete network from separate CSV files
pub fn load_network_from_csv<P: AsRef<Path>>(
    stops_path: P,
    routes_path: P,
    transfers_path: Option<P>,
    geo_config: Option<&GeoConfig>,
) -> Result<Network, Box<dyn std::error::Error>> {
    let stops = load_stops_csv(stops_path)?;
    let routes = load_routes_csv(routes_path)?;
    
    let mut transfers = if let Some(transfers_path) = transfers_path {
        load_transfers_csv(transfers_path)?
    } else {
        Vec::new()
    };
    
    // Generate walking transfers if geo_config is provided
    if let Some(config) = geo_config {
        let mut generated_transfers = config.generate_transfers(&stops);
        transfers.append(&mut generated_transfers);
    }
    
    Ok(Network::new(stops, routes, transfers))
}

/// NetworkBuilder for fluent construction
pub struct NetworkBuilder {
    stops: Vec<Stop>,
    routes: Vec<Route>,
    transfers: Vec<Transfer>,
    geo_config: Option<GeoConfig>,
}

impl NetworkBuilder {
    pub fn new() -> Self {
        Self {
            stops: Vec::new(),
            routes: Vec::new(),
            transfers: Vec::new(),
            geo_config: None,
        }
    }
    
    pub fn with_stops_csv<P: AsRef<Path>>(mut self, path: P) -> Result<Self, Box<dyn std::error::Error>> {
        self.stops = load_stops_csv(path)?;
        Ok(self)
    }
    
    pub fn with_routes_csv<P: AsRef<Path>>(mut self, path: P) -> Result<Self, Box<dyn std::error::Error>> {
        self.routes = load_routes_csv(path)?;
        Ok(self)
    }
    
    pub fn with_transfers_csv<P: AsRef<Path>>(mut self, path: P) -> Result<Self, Box<dyn std::error::Error>> {
        self.transfers = load_transfers_csv(path)?;
        Ok(self)
    }
    
    pub fn with_geo_config(mut self, config: GeoConfig) -> Self {
        self.geo_config = Some(config);
        self
    }
    
    pub fn build(mut self) -> Network {
        // Generate walking transfers if config is provided
        if let Some(config) = &self.geo_config {
            let mut generated_transfers = config.generate_transfers(&self.stops);
            self.transfers.append(&mut generated_transfers);
        }
        
        Network::new(self.stops, self.routes, self.transfers)
    }
}

impl Default for NetworkBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_csv_round_trip_stops() {
        let stops = vec![
            Stop::new(1, "Station A".to_string(), 35.0, 139.0),
            Stop::new(2, "Station B".to_string(), 35.1, 139.1),
        ];
        
        // Create temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "id,name,lat,lon").unwrap();
        writeln!(temp_file, "1,Station A,35.0,139.0").unwrap();
        writeln!(temp_file, "2,Station B,35.1,139.1").unwrap();
        
        // Test loading
        let loaded_stops = load_stops_csv(temp_file.path()).unwrap();
        
        assert_eq!(loaded_stops.len(), 2);
        assert_eq!(loaded_stops[0].id, 1);
        assert_eq!(loaded_stops[0].name, "Station A");
        assert_eq!(loaded_stops[0].lat, 35.0);
    }

    #[test]
    fn test_csv_round_trip_routes() {
        // Create temporary file
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "id,name,stops,travel_times").unwrap();
        writeln!(temp_file, "1,Route 1,\"1,2,3\",\"600,600\"").unwrap();
        
        // Test loading
        let loaded_routes = load_routes_csv(temp_file.path()).unwrap();
        
        assert_eq!(loaded_routes.len(), 1);
        assert_eq!(loaded_routes[0].id, 1);
        assert_eq!(loaded_routes[0].stops, vec![1, 2, 3]);
        assert_eq!(loaded_routes[0].travel_times, vec![600, 600]);
    }
}