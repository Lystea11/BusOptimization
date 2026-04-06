use bus_raptor::data::structures::{Stop, Route, Transfer, Network};
use bus_raptor::data::loader::{load_network_from_csv, save_stops_csv, save_routes_csv};
use bus_raptor::raptor::backward::BackwardRaptor;
use bus_raptor::raptor::journey::parse_time;
use bus_raptor::geo::haversine::{GeoConfig, calculate_distance};
use tempfile::{NamedTempFile, TempDir};
use std::io::Write;

#[test]
fn test_complete_routing_scenario() {
    // Create a simple test network: A -> B -> C with walking transfer B -> D
    let stops = vec![
        Stop::new(1, "Stop A".to_string(), 35.0, 139.0),
        Stop::new(2, "Stop B".to_string(), 35.001, 139.001), // ~157m from A
        Stop::new(3, "Stop C".to_string(), 35.002, 139.002), // ~157m from B  
        Stop::new(4, "Stop D".to_string(), 35.001, 139.0015), // ~55m from B
    ];
    
    let routes = vec![
        Route::new(1, "Route 1".to_string(), vec![1, 2, 3], vec![300, 300]), // 5 minutes each segment
    ];
    
    let geo_config = GeoConfig::new().with_max_walking_distance_m(100.0);
    let transfers = geo_config.generate_transfers(&stops);
    
    let network = Network::new(stops, routes, transfers);
    let raptor = BackwardRaptor::new(network, 3);
    
    // Test route to Stop C arriving at 8:00 AM (28800 seconds)
    let journey = raptor.find_route(None, 3, 28800);
    
    assert!(journey.is_some());
    let journey = journey.unwrap();
    assert_eq!(journey.arrival_time, 28800);
    assert!(journey.departure_time <= 28800);
    assert!(journey.total_time >= 600); // At least 10 minutes for the route
}

#[test]
fn test_csv_data_loading() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create test stops file
    let stops_file = temp_dir.path().join("stops.csv");
    let mut file = std::fs::File::create(&stops_file).unwrap();
    writeln!(file, "id,name,lat,lon").unwrap();
    writeln!(file, "1,Station A,35.0,139.0").unwrap();
    writeln!(file, "2,Station B,35.1,139.1").unwrap();
    
    // Create test routes file
    let routes_file = temp_dir.path().join("routes.csv");
    let mut file = std::fs::File::create(&routes_file).unwrap();
    writeln!(file, "id,name,stops,travel_times").unwrap();
    writeln!(file, "1,Route 1,\"1,2\",\"600\"").unwrap();
    
    // Load network
    let network = load_network_from_csv(
        &stops_file,
        &routes_file,
        None::<&std::path::PathBuf>,
        None,
    ).unwrap();
    
    assert_eq!(network.stops.len(), 2);
    assert_eq!(network.routes.len(), 1);
    assert_eq!(network.routes[0].stops, vec![1, 2]);
    assert_eq!(network.routes[0].travel_times, vec![600]);
}

#[test]
fn test_geospatial_calculations() {
    // Test Haversine distance calculation
    let distance = calculate_distance(35.0, 139.0, 35.001, 139.001);
    
    // Should be approximately 144 meters (using Haversine formula)
    assert!(distance > 130.0 && distance < 160.0, "Distance: {}", distance);
}

#[test]
fn test_walking_transfer_generation() {
    let stops = vec![
        Stop::new(1, "Stop A".to_string(), 35.0, 139.0),
        Stop::new(2, "Stop B".to_string(), 35.001, 139.001), // ~157m away
        Stop::new(3, "Stop C".to_string(), 35.01, 139.01),   // ~1.57km away
    ];
    
    let config = GeoConfig::new().with_max_walking_distance_m(200.0);
    let transfers = config.generate_transfers(&stops);
    
    // Should generate transfers between A and B (both directions) but not with C
    assert_eq!(transfers.len(), 2); // A->B and B->A
    
    assert!(transfers.iter().any(|t| t.from_stop == 1 && t.to_stop == 2));
    assert!(transfers.iter().any(|t| t.from_stop == 2 && t.to_stop == 1));
    assert!(!transfers.iter().any(|t| t.from_stop == 1 && t.to_stop == 3));
}

#[test]
fn test_time_parsing() {
    assert_eq!(parse_time("00:00").unwrap(), 0);
    assert_eq!(parse_time("08:00").unwrap(), 28800); // 8 * 3600
    assert_eq!(parse_time("12:30").unwrap(), 45000); // 12 * 3600 + 30 * 60
    assert_eq!(parse_time("23:59").unwrap(), 86340);
    
    // Test invalid inputs
    assert!(parse_time("25:00").is_err()); // Invalid hour
    assert!(parse_time("12:60").is_err()); // Invalid minute
    assert!(parse_time("abc").is_err());   // Invalid format
}

#[test]
fn test_journey_validation_and_formatting() {
    let stops = vec![
        Stop::new(1, "Stop A".to_string(), 35.0, 139.0),
        Stop::new(2, "Stop B".to_string(), 35.001, 139.001),
        Stop::new(3, "Stop C".to_string(), 35.002, 139.002),
    ];
    
    let routes = vec![
        Route::new(1, "Route 1".to_string(), vec![1, 2], vec![600]),
        Route::new(2, "Route 2".to_string(), vec![2, 3], vec![300]),
    ];
    
    let network = Network::new(stops, routes, vec![]);
    let mut journey = bus_raptor::data::structures::Journey::new();
    
    // Add journey legs
    journey.add_bus_leg(1, 1, 2, 28800, 29400); // 8:00 - 8:10
    journey.add_walk_leg(2, 2, 60);              // 1 minute walk (connection)
    journey.add_bus_leg(2, 2, 3, 29460, 29760); // 8:11 - 8:16
    
    journey.finalize(28800, 29760);
    
    assert!(journey.is_valid());
    assert_eq!(journey.get_origin_stop(), Some(1));
    assert_eq!(journey.get_destination_stop(), Some(3));
    assert_eq!(journey.total_time, 960); // 16 minutes
    assert_eq!(journey.num_transfers, 1);
    
    // Test formatting
    let formatted = journey.format_journey(&network);
    assert!(formatted.contains("Stop A"));
    assert!(formatted.contains("Stop C"));
    assert!(formatted.contains("Route 1"));
}

#[test]
fn test_network_indexing() {
    let stops = vec![
        Stop::new(101, "Stop A".to_string(), 35.0, 139.0),
        Stop::new(205, "Stop B".to_string(), 35.1, 139.1),
    ];
    
    let routes = vec![
        Route::new(301, "Route X".to_string(), vec![101, 205], vec![600]),
    ];
    
    let network = Network::new(stops, routes, vec![]);
    
    // Test stop indexing
    assert_eq!(network.get_stop_index(101), Some(0));
    assert_eq!(network.get_stop_index(205), Some(1));
    assert_eq!(network.get_stop_index(999), None);
    
    // Test route indexing
    assert_eq!(network.get_route_index(301), Some(0));
    assert_eq!(network.get_route_index(999), None);
    
    // Test stop retrieval
    let stop_a = network.get_stop(101).unwrap();
    assert_eq!(stop_a.name, "Stop A");
    
    // Test routes for stop
    let routes_for_stop = network.get_routes_for_stop(101).unwrap();
    assert_eq!(routes_for_stop.len(), 1);
    assert_eq!(routes_for_stop[0], 0); // Index of route 301
}

#[test]
fn test_route_travel_time_calculations() {
    let route = Route::new(
        1, 
        "Test Route".to_string(), 
        vec![1, 2, 3, 4], 
        vec![300, 600, 900] // 5min, 10min, 15min
    );
    
    // Test cumulative times
    assert_eq!(route.cumulative_time_to(0), 0);
    assert_eq!(route.cumulative_time_to(1), 300);  // 5min
    assert_eq!(route.cumulative_time_to(2), 900);  // 5+10=15min
    assert_eq!(route.cumulative_time_to(3), 1800); // 5+10+15=30min
    
    // Test travel time between stops
    assert_eq!(route.travel_time_between(0, 1), Some(300));  // Stop 0->1: 5min
    assert_eq!(route.travel_time_between(0, 2), Some(900));  // Stop 0->2: 15min
    assert_eq!(route.travel_time_between(1, 3), Some(1500)); // Stop 1->3: 25min
    assert_eq!(route.travel_time_between(2, 1), None);       // Backwards: invalid
    assert_eq!(route.travel_time_between(0, 4), None);       // Out of bounds
}

#[test]
fn test_multi_round_routing() {
    // Create a network requiring transfers: A->B, C->D, with walking B<->C
    let stops = vec![
        Stop::new(1, "Stop A".to_string(), 35.0, 139.0),
        Stop::new(2, "Stop B".to_string(), 35.001, 139.001),
        Stop::new(3, "Stop C".to_string(), 35.001, 139.0015), // Close to B for walking
        Stop::new(4, "Stop D".to_string(), 35.002, 139.002),
    ];
    
    let routes = vec![
        Route::new(1, "Route AB".to_string(), vec![1, 2], vec![600]), // A->B: 10min
        Route::new(2, "Route CD".to_string(), vec![3, 4], vec![300]), // C->D: 5min
    ];
    
    // Add walking transfer B<->C
    let transfers = vec![
        Transfer::new(2, 3, 120, 100.0), // B->C: 2min walk
        Transfer::new(3, 2, 120, 100.0), // C->B: 2min walk
    ];
    
    let network = Network::new(stops, routes, transfers);
    let raptor = BackwardRaptor::new(network, 4); // Allow up to 3 transfers
    
    // Find route from A to D
    let journey = raptor.find_route(None, 4, 28800); // Arrive at 8:00
    
    assert!(journey.is_some());
    let journey = journey.unwrap();
    
    // Should have bus + walk + bus legs
    assert!(journey.legs.len() >= 3);
    assert!(journey.num_transfers >= 1);
    assert_eq!(journey.arrival_time, 28800);
    
    // Total travel time should be at least 17 minutes (10+2+5)
    assert!(journey.total_time >= 1020);
}