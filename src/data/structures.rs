use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type StopId = u32;
pub type RouteId = u32;
pub type Time = u32; // Seconds since midnight

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stop {
    pub id: StopId,
    pub name: String,
    pub lat: f32,  // f32 for cache efficiency
    pub lon: f32,
}

impl Stop {
    pub fn new(id: StopId, name: String, lat: f32, lon: f32) -> Self {
        Self { id, name, lat, lon }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub id: RouteId,
    pub name: String,
    pub stops: Vec<StopId>,        // Ordered stop IDs along the route
    pub travel_times: Vec<Time>,   // Travel time between consecutive stops (len = stops.len() - 1)
}

impl Route {
    pub fn new(id: RouteId, name: String, stops: Vec<StopId>, travel_times: Vec<Time>) -> Self {
        assert_eq!(stops.len() - 1, travel_times.len(), 
                  "Travel times must be one less than stops count");
        Self { id, name, stops, travel_times }
    }

    /// Get cumulative travel time from first stop to stop at given index
    pub fn cumulative_time_to(&self, stop_index: usize) -> Time {
        self.travel_times.iter().take(stop_index).sum()
    }

    /// Get travel time between two stop indices on this route
    pub fn travel_time_between(&self, from_index: usize, to_index: usize) -> Option<Time> {
        if from_index >= to_index || to_index >= self.stops.len() {
            return None;
        }
        Some(self.travel_times[from_index..to_index].iter().sum())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    pub from_stop: StopId,
    pub to_stop: StopId,
    pub walk_time: Time,    // Walking time in seconds
    pub distance: f32,      // Distance in meters
}

impl Transfer {
    pub fn new(from_stop: StopId, to_stop: StopId, walk_time: Time, distance: f32) -> Self {
        Self { from_stop, to_stop, walk_time, distance }
    }
}

#[derive(Debug, Clone)]
pub struct Network {
    pub stops: Vec<Stop>,
    pub routes: Vec<Route>,
    pub transfers: Vec<Transfer>,
    
    // Index mappings for O(1) lookups
    pub stop_id_to_index: HashMap<StopId, usize>,
    pub route_id_to_index: HashMap<RouteId, usize>,
    
    // Routes serving each stop (stop_index -> route_indices)
    pub stop_routes: Vec<Vec<usize>>,
}

impl Network {
    pub fn new(stops: Vec<Stop>, routes: Vec<Route>, transfers: Vec<Transfer>) -> Self {
        // Build index mappings
        let stop_id_to_index: HashMap<StopId, usize> = stops
            .iter()
            .enumerate()
            .map(|(i, stop)| (stop.id, i))
            .collect();

        let route_id_to_index: HashMap<RouteId, usize> = routes
            .iter()
            .enumerate()
            .map(|(i, route)| (route.id, i))
            .collect();

        // Build stop -> routes mapping
        let mut stop_routes: Vec<Vec<usize>> = vec![Vec::new(); stops.len()];
        for (route_idx, route) in routes.iter().enumerate() {
            for &stop_id in &route.stops {
                if let Some(&stop_idx) = stop_id_to_index.get(&stop_id) {
                    stop_routes[stop_idx].push(route_idx);
                }
            }
        }

        Self {
            stops,
            routes,
            transfers,
            stop_id_to_index,
            route_id_to_index,
            stop_routes,
        }
    }

    pub fn get_stop_index(&self, stop_id: StopId) -> Option<usize> {
        self.stop_id_to_index.get(&stop_id).copied()
    }

    pub fn get_route_index(&self, route_id: RouteId) -> Option<usize> {
        self.route_id_to_index.get(&route_id).copied()
    }

    pub fn get_stop(&self, stop_id: StopId) -> Option<&Stop> {
        self.get_stop_index(stop_id)
            .map(|idx| &self.stops[idx])
    }

    pub fn get_route(&self, route_id: RouteId) -> Option<&Route> {
        self.get_route_index(route_id)
            .map(|idx| &self.routes[idx])
    }

    pub fn get_stop_by_name(&self, stop_name: &str) -> Option<StopId> {
        self.stops.iter().find(|s| s.name.trim().eq_ignore_ascii_case(stop_name.trim())).map(|s| s.id)
    }

    /// Get all transfers from a specific stop
    pub fn get_transfers_from(&self, stop_id: StopId) -> impl Iterator<Item = &Transfer> {
        self.transfers.iter().filter(move |t| t.from_stop == stop_id)
    }

    /// Get all routes serving a specific stop
    pub fn get_routes_for_stop(&self, stop_id: StopId) -> Option<&Vec<usize>> {
        self.get_stop_index(stop_id)
            .map(|idx| &self.stop_routes[idx])
    }
}

#[derive(Debug, Clone)]
pub struct Journey {
    pub legs: Vec<JourneyLeg>,
    pub total_time: Time,
    pub num_transfers: usize,
    pub departure_time: Time,
    pub arrival_time: Time,
}

#[derive(Debug, Clone)]
pub enum JourneyLeg {
    Bus {
        route_id: RouteId,
        from_stop: StopId,
        to_stop: StopId,
        departure_time: Time,
        arrival_time: Time,
    },
    Walk {
        from_stop: StopId,
        to_stop: StopId,
        duration: Time,
    },
}

impl Journey {
    pub fn new() -> Self {
        Self {
            legs: Vec::new(),
            total_time: 0,
            num_transfers: 0,
            departure_time: 0,
            arrival_time: 0,
        }
    }

    pub fn add_bus_leg(&mut self, route_id: RouteId, from_stop: StopId, to_stop: StopId, 
                       departure_time: Time, arrival_time: Time) {
        self.legs.push(JourneyLeg::Bus {
            route_id,
            from_stop,
            to_stop,
            departure_time,
            arrival_time,
        });
    }

    pub fn add_walk_leg(&mut self, from_stop: StopId, to_stop: StopId, duration: Time) {
        self.legs.push(JourneyLeg::Walk {
            from_stop,
            to_stop,
            duration,
        });
        self.num_transfers += 1;
    }

    pub fn finalize(&mut self, departure_time: Time, arrival_time: Time) {
        self.departure_time = departure_time;
        self.arrival_time = arrival_time;
        self.total_time = arrival_time - departure_time;
    }
}