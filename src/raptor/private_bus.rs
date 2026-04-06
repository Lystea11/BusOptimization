use crate::data::structures::{Network, Journey, StopId, Time};

/// A planner for a single private bus journey with multiple stops.
#[derive(Debug)]
pub struct PrivateBusPlanner {
    raptor: crate::raptor::backward::BackwardRaptor,
}

impl PrivateBusPlanner {
    /// Creates a new `PrivateBusPlanner`.
    pub fn new(network: Network, max_rounds: usize) -> Self {
        Self {
            raptor: crate::raptor::backward::BackwardRaptor::new(network, max_rounds),
        }
    }

    /// Finds a journey that visits a sequence of stops, ending at the destination by a specific time.
    /// The stops are visited in the order they are provided.
    pub fn find_journey(
        &self,
        stops: Vec<StopId>,
        destination_name: &str,
        arrival_time: Time,
    ) -> Result<Journey, String> {
        if stops.is_empty() {
            return Err("At least one stop must be provided.".to_string());
        }

        let mut final_journey = Journey::new();
        let mut current_arrival_time = arrival_time;

        // Get the final destination ID from its name
        let destination_id = self
            .raptor
            .network
            .get_stop_by_name(destination_name)
            .ok_or_else(|| format!("Destination '{}' not found.", destination_name))?;

        let mut journey_segments = Vec::new();
        let all_stops = [stops.as_slice(), &[destination_id]].concat();

        // Iterate backward from the second-to-last stop
        for i in (0..all_stops.len() - 1).rev() {
            let from_stop = all_stops[i];
            let to_stop = all_stops[i + 1];

            // Find a route for the current segment
            let segment_journey = self.raptor.find_route(Some(from_stop), to_stop, current_arrival_time)
                .ok_or_else(|| format!("No route found from stop {} to {}", from_stop, to_stop))?;
            
            // The new arrival time for the previous segment is the departure time of the current one
            current_arrival_time = segment_journey.departure_time;
            journey_segments.push(segment_journey);
        }

        // Reverse the segments to get the correct chronological order
        journey_segments.reverse();

        // Combine the segments into a single journey
        for segment in journey_segments {
            final_journey.legs.extend(segment.legs);
        }

        if let Some(first_leg) = final_journey.legs.first() {
            final_journey.departure_time = match first_leg {
                crate::data::structures::JourneyLeg::Bus { departure_time, .. } => *departure_time,
                crate::data::structures::JourneyLeg::Walk { .. } => 0, // Assuming walk has no departure time
            };
        }
        final_journey.arrival_time = arrival_time;

        Ok(final_journey)
    }
}
