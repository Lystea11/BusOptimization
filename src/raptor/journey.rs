use crate::data::structures::{Journey, JourneyLeg, StopId, RouteId, Time, Network};

impl Journey {
    /// Format journey for display
    pub fn format_journey(&self, network: &Network) -> String {
        let mut output = String::new();
        
        output.push_str(&format!(
            "Journey from {} to {}\n",
            self.format_time(self.departure_time),
            self.format_time(self.arrival_time)
        ));
        
        output.push_str(&format!(
            "Total time: {} minutes, Transfers: {}\n\n",
            self.total_time / 60,
            self.num_transfers
        ));

        for (i, leg) in self.legs.iter().enumerate() {
            match leg {
                JourneyLeg::Bus { route_id, from_stop, to_stop, departure_time, arrival_time } => {
                    let from_name = network.get_stop(*from_stop)
                        .map(|s| s.name.as_str())
                        .unwrap_or("Unknown");
                    let to_name = network.get_stop(*to_stop)
                        .map(|s| s.name.as_str())
                        .unwrap_or("Unknown");
                    let route_name = network.get_route(*route_id)
                        .map(|r| r.name.as_str())
                        .unwrap_or("Unknown Route");
                    
                    output.push_str(&format!(
                        "{}. Bus - {} from {} to {}\n",
                        i + 1, route_name, from_name, to_name
                    ));
                    output.push_str(&format!(
                        "   Depart: {} -> Arrive: {} ({} min)\n",
                        self.format_time(*departure_time),
                        self.format_time(*arrival_time),
                        (arrival_time - departure_time) / 60
                    ));
                }
                JourneyLeg::Walk { from_stop, to_stop, duration } => {
                    let from_name = network.get_stop(*from_stop)
                        .map(|s| s.name.as_str())
                        .unwrap_or("Unknown");
                    let to_name = network.get_stop(*to_stop)
                        .map(|s| s.name.as_str())
                        .unwrap_or("Unknown");
                    
                    output.push_str(&format!(
                        "{}. Walk from {} to {} ({} min)\n",
                        i + 1, from_name, to_name, duration / 60
                    ));
                }
            }
        }
        
        output
    }

    /// Format time as HH:MM
    fn format_time(&self, seconds: Time) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        format!("{:02}:{:02}", hours, minutes)
    }

    /// Get the starting stop of the journey
    pub fn get_origin_stop(&self) -> Option<StopId> {
        match self.legs.first()? {
            JourneyLeg::Bus { from_stop, .. } => Some(*from_stop),
            JourneyLeg::Walk { from_stop, .. } => Some(*from_stop),
        }
    }

    /// Get the ending stop of the journey
    pub fn get_destination_stop(&self) -> Option<StopId> {
        match self.legs.last()? {
            JourneyLeg::Bus { to_stop, .. } => Some(*to_stop),
            JourneyLeg::Walk { to_stop, .. } => Some(*to_stop),
        }
    }

    /// Check if journey is valid (continuous stops)
    pub fn is_valid(&self) -> bool {
        if self.legs.is_empty() {
            return false;
        }

        for window in self.legs.windows(2) {
            let end_of_first = match &window[0] {
                JourneyLeg::Bus { to_stop, .. } => *to_stop,
                JourneyLeg::Walk { to_stop, .. } => *to_stop,
            };
            
            let start_of_second = match &window[1] {
                JourneyLeg::Bus { from_stop, .. } => *from_stop,
                JourneyLeg::Walk { from_stop, .. } => *from_stop,
            };
            
            if end_of_first != start_of_second {
                return false;
            }
        }
        
        true
    }

    /// Get all stops visited in order
    pub fn get_stops_visited(&self) -> Vec<StopId> {
        let mut stops = Vec::new();
        
        for leg in &self.legs {
            match leg {
                JourneyLeg::Bus { from_stop, to_stop, .. } => {
                    if stops.is_empty() {
                        stops.push(*from_stop);
                    }
                    stops.push(*to_stop);
                }
                JourneyLeg::Walk { from_stop, to_stop, .. } => {
                    if stops.is_empty() {
                        stops.push(*from_stop);
                    }
                    stops.push(*to_stop);
                }
            }
        }
        
        stops
    }

    /// Get summary statistics
    pub fn get_summary(&self) -> JourneySummary {
        let mut bus_time = 0;
        let mut walk_time = 0;
        let mut bus_legs = 0;
        let mut walk_legs = 0;

        for leg in &self.legs {
            match leg {
                JourneyLeg::Bus { departure_time, arrival_time, .. } => {
                    bus_time += arrival_time - departure_time;
                    bus_legs += 1;
                }
                JourneyLeg::Walk { duration, .. } => {
                    walk_time += duration;
                    walk_legs += 1;
                }
            }
        }

        JourneySummary {
            total_time: self.total_time,
            bus_time,
            walk_time,
            bus_legs,
            walk_legs,
            total_transfers: self.num_transfers,
            departure_time: self.departure_time,
            arrival_time: self.arrival_time,
        }
    }
}

#[derive(Debug, Clone)]
pub struct JourneySummary {
    pub total_time: Time,
    pub bus_time: Time,
    pub walk_time: Time,
    pub bus_legs: usize,
    pub walk_legs: usize,
    pub total_transfers: usize,
    pub departure_time: Time,
    pub arrival_time: Time,
}

impl JourneySummary {
    pub fn format_summary(&self) -> String {
        format!(
            "Journey Summary:
  Total Time: {} minutes
  Bus Time: {} minutes ({} legs)
  Walk Time: {} minutes ({} legs)
  Transfers: {}
  Departure: {}
  Arrival: {}",
            self.total_time / 60,
            self.bus_time / 60,
            self.bus_legs,
            self.walk_time / 60,
            self.walk_legs,
            self.total_transfers,
            self.format_time(self.departure_time),
            self.format_time(self.arrival_time)
        )
    }

    fn format_time(&self, seconds: Time) -> String {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        format!("{:02}:{:02}", hours, minutes)
    }
}

/// Parse time string (HH:MM) to seconds since midnight
pub fn parse_time(time_str: &str) -> Result<Time, String> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 2 {
        return Err("Time must be in HH:MM format".to_string());
    }

    let hours: u32 = parts[0].parse()
        .map_err(|_| "Invalid hours")?;
    let minutes: u32 = parts[1].parse()
        .map_err(|_| "Invalid minutes")?;

    if hours > 23 || minutes > 59 {
        return Err("Invalid time values".to_string());
    }

    Ok(hours * 3600 + minutes * 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time() {
        assert_eq!(parse_time("08:00").unwrap(), 28800); // 8 * 3600
        assert_eq!(parse_time("12:30").unwrap(), 45000); // 12 * 3600 + 30 * 60
        assert_eq!(parse_time("00:00").unwrap(), 0);
        
        assert!(parse_time("25:00").is_err()); // Invalid hour
        assert!(parse_time("12:60").is_err()); // Invalid minute
        assert!(parse_time("12").is_err());    // Missing minute
    }

    #[test]
    fn test_journey_validation() {
        let mut journey = Journey::new();
        journey.add_bus_leg(1, 1, 2, 1000, 1600);
        journey.add_walk_leg(2, 3, 300);
        journey.add_bus_leg(2, 3, 4, 1900, 2400);
        journey.finalize(1000, 2400);

        assert!(journey.is_valid());
        assert_eq!(journey.get_origin_stop(), Some(1));
        assert_eq!(journey.get_destination_stop(), Some(4));
        assert_eq!(journey.get_stops_visited(), vec![1, 2, 3, 4]);
    }
}