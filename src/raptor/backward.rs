use crate::data::structures::{Network, Journey, JourneyLeg, StopId, RouteId, Time};
use bitvec::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct BackwardRaptor {
    pub network: Network,
    max_rounds: usize,
    max_departure_delay: Time,
}

#[derive(Debug, Clone)]
struct RaptorState {
    // Best departure time to reach destination for each round (stop_index -> time)
    best_times: Vec<Vec<Time>>,
    // Parent information for journey reconstruction (stop_index -> (parent_stop, route_id, round))
    parents: Vec<HashMap<usize, (usize, RouteId, usize)>>,
    // Marked stops for current round
    marked_stops: BitVec,
    // Routes to process in current round
    marked_routes: BitVec,
}

impl BackwardRaptor {
    pub fn new(network: Network, max_rounds: usize) -> Self {
        Self {
            network,
            max_rounds,
            max_departure_delay: 3600, // 1 hour default
        }
    }

    pub fn with_max_departure_delay(mut self, max_delay: Time) -> Self {
        self.max_departure_delay = max_delay;
        self
    }

    pub fn find_route(&self, from_stop_id: Option<StopId>, destination_id: StopId, target_arrival_time: Time) -> Option<Journey> {
        let destination_idx = self.network.get_stop_index(destination_id)?;
        let from_stop_idx = from_stop_id.and_then(|id| self.network.get_stop_index(id));

        // Initialize state
        let num_stops = self.network.stops.len();
        let num_routes = self.network.routes.len();
        
        let mut state = RaptorState {
            best_times: vec![vec![Time::MAX; num_stops]; self.max_rounds + 1],
            parents: vec![HashMap::new(); num_stops],
            marked_stops: bitvec![0; num_stops],
            marked_routes: bitvec![0; num_routes],
        };

        // Initialize destination
        state.best_times[0][destination_idx] = target_arrival_time;
        state.marked_stops.set(destination_idx, true);

        // Main RAPTOR rounds
        for round in 1..=self.max_rounds {
            let mut improved_stops = bitvec![0; num_stops];
            self.mark_routes_for_round(&mut state, round - 1);

            if state.marked_routes.not_any() {
                break;
            }

            for route_idx in state.marked_routes.iter_ones() {
                let improvements = self.process_route_backward(&state, route_idx, round);
                for (stop_idx, new_time, parent_info) in improvements {
                    if new_time < state.best_times[round][stop_idx] {
                        state.best_times[round][stop_idx] = new_time;
                        state.parents[stop_idx].insert(round, parent_info);
                        improved_stops.set(stop_idx, true);
                    }
                }
            }

            self.process_transfers_backward(&mut state, round, &mut improved_stops);

            state.marked_stops = improved_stops;
            if state.marked_stops.not_any() {
                break;
            }
        }

        self.find_best_journey(&state, from_stop_idx, destination_idx, target_arrival_time)
    }

    /// Mark routes that need to be processed based on improved stops
    fn mark_routes_for_round(&self, state: &mut RaptorState, prev_round: usize) {
        state.marked_routes.fill(false);
        
        for stop_idx in state.marked_stops.iter_ones() {
            if let Some(routes) = self.network.stop_routes.get(stop_idx) {
                for &route_idx in routes {
                    state.marked_routes.set(route_idx, true);
                }
            }
        }
    }

    /// Process a single route backward to find earlier departure times
    fn process_route_backward(
        &self, 
        state: &RaptorState, 
        route_idx: usize, 
        round: usize
    ) -> Vec<(usize, Time, (usize, RouteId, usize))> {
        let route = &self.network.routes[route_idx];
        let mut improvements = Vec::new();
        
        // For backward search, we scan from the end of the route to the beginning
        let mut best_arrival_time = Time::MAX;
        let mut best_stop_info: Option<(usize, usize)> = None; // (stop_idx, position_on_route)

        // Scan route backward (from end to start)
        for (route_pos, &stop_id) in route.stops.iter().enumerate().rev() {
            if let Some(stop_idx) = self.network.get_stop_index(stop_id) {
                // Check if we can reach destination from this stop
                let current_time = state.best_times[round - 1][stop_idx];
                
                if current_time < best_arrival_time {
                    best_arrival_time = current_time;
                    best_stop_info = Some((stop_idx, route_pos));
                }
                
                // If we have a valid path to destination, calculate departure time for this stop
                if let Some((arrival_stop_idx, arrival_pos)) = best_stop_info {
                    if route_pos < arrival_pos {
                        // Calculate travel time from current position to arrival position
                        if let Some(travel_time) = route.travel_time_between(route_pos, arrival_pos) {
                            let departure_time = best_arrival_time.saturating_sub(travel_time);
                            
                            // Check if this is an improvement
                            if departure_time < state.best_times[round][stop_idx] {
                                improvements.push((
                                    stop_idx,
                                    departure_time,
                                    (arrival_stop_idx, route.id, round)
                                ));
                            }
                        }
                    }
                }
            }
        }
        
        improvements
    }

    /// Process walking transfers backward
    fn process_transfers_backward(
        &self, 
        state: &mut RaptorState, 
        round: usize, 
        improved_stops: &mut BitVec
    ) {
        let current_improvements: Vec<_> = improved_stops.iter_ones().collect();
        
        for stop_idx in current_improvements {
            let stop_id = self.network.stops[stop_idx].id;
            let arrival_time = state.best_times[round][stop_idx];
            
            // Check all walking transfers TO this stop (backward search)
            for transfer in &self.network.transfers {
                if transfer.to_stop == stop_id {
                    if let Some(from_stop_idx) = self.network.get_stop_index(transfer.from_stop) {
                        let departure_time = arrival_time.saturating_sub(transfer.walk_time);
                        
                        // Check if this walking transfer provides improvement
                        if departure_time < state.best_times[round][from_stop_idx] {
                            state.best_times[round][from_stop_idx] = departure_time;
                            state.parents[from_stop_idx].insert(round, (stop_idx, u32::MAX, round)); // u32::MAX indicates walking
                            improved_stops.set(from_stop_idx, true);
                        }
                    }
                }
            }
        }
    }

    /// Find the best overall journey by looking across all rounds
    fn find_best_journey(
        &self, 
        state: &RaptorState, 
        from_stop_idx: Option<usize>,
        destination_idx: usize, 
        target_arrival_time: Time
    ) -> Option<Journey> {
        let mut best_departure_time = Time::MAX;
        let mut best_round = 0;
        let mut best_start_stop = None;

        if let Some(from_idx) = from_stop_idx {
            // If a specific start stop is provided, only consider that stop
            for round in 1..=self.max_rounds {
                let departure_time = state.best_times[round][from_idx];
                if departure_time < best_departure_time {
                    best_departure_time = departure_time;
                    best_round = round;
                    best_start_stop = Some(from_idx);
                }
            }
        } else {
            // Otherwise, find the best departure from any stop
            for round in 1..=self.max_rounds {
                for (stop_idx, &departure_time) in state.best_times[round].iter().enumerate() {
                    if departure_time < best_departure_time {
                        best_departure_time = departure_time;
                        best_round = round;
                        best_start_stop = Some(stop_idx);
                    }
                }
            }
        }

        if let Some(mut current_stop_idx) = best_start_stop {
            let mut journey = Journey::new();
            let mut current_round = best_round;

            while current_round > 0 {
                if let Some(&(parent_stop_idx, route_id, _round)) = state.parents[current_stop_idx].get(&current_round) {
                    let from_stop_id = self.network.stops[current_stop_idx].id;
                    let to_stop_id = self.network.stops[parent_stop_idx].id;

                    if route_id == u32::MAX {
                        // Walking transfer: parent is in the same round
                        let departure_time = state.best_times[current_round][current_stop_idx];
                        let arrival_time = state.best_times[current_round][parent_stop_idx];
                        let duration = arrival_time.saturating_sub(departure_time);
                        journey.add_walk_leg(from_stop_id, to_stop_id, duration);
                        current_stop_idx = parent_stop_idx;
                        // Don't decrement round — walking is within the same round
                    } else {
                        // Bus leg: parent is in the previous round
                        let departure_time = state.best_times[current_round][current_stop_idx];
                        let arrival_time = state.best_times[current_round - 1][parent_stop_idx];
                        journey.add_bus_leg(route_id, from_stop_id, to_stop_id, departure_time, arrival_time);
                        current_stop_idx = parent_stop_idx;
                        current_round -= 1;
                    }
                } else {
                    break; // No parent, end of journey
                }
            }

            journey.legs.reverse(); // Legs are added backward
            journey.finalize(best_departure_time, target_arrival_time);
            Some(journey)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::structures::{Stop, Route, Transfer};

    #[test]
    fn test_backward_raptor_simple() {
        // Create a simple test network: A -> B -> C
        let stops = vec![
            Stop::new(1, "Stop A".to_string(), 35.0, 139.0),
            Stop::new(2, "Stop B".to_string(), 35.1, 139.1), 
            Stop::new(3, "Stop C".to_string(), 35.2, 139.2),
        ];
        
        let routes = vec![
            Route::new(1, "Route 1".to_string(), vec![1, 2, 3], vec![600, 600]), // 10 minutes each segment
        ];
        
        let transfers = vec![]; // No walking transfers
        
        let network = Network::new(stops, routes, transfers);
        let raptor = BackwardRaptor::new(network, 3);
        
        // Find route to stop 3 (C) arriving by 08:00 (28800 seconds)
        let result = raptor.find_route(None, 3, 28800);
        
        assert!(result.is_some());
        let journey = result.unwrap();
        assert!(journey.departure_time <= 28800);
        assert_eq!(journey.arrival_time, 28800);
    }
}