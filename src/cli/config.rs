use clap::{Arg, ArgAction, Command};
use crate::geo::haversine::GeoConfig;

#[derive(Debug, Clone)]
pub enum CliCommand {
    Public(CliConfig),
    Private(PrivateBusConfig),
}

#[derive(Debug, Clone)]
pub struct PrivateBusConfig {
    pub stops_file: String,
    pub routes_file: String,
    pub stops: Vec<String>,
    pub destination: String,
    pub arrive_by: String,
    pub output_format: OutputFormat,
    pub verbose: bool,
    pub output_geojson: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CliConfig {
    pub stops_file: String,
    pub routes_file: String,
    pub transfers_file: Option<String>,
    pub destination: String,
    pub arrive_by: String,
    pub max_transfers: Option<usize>,
    pub max_walking_distance: Option<f32>,
    pub walking_speed_kmh: Option<f32>,
    pub bus_speed_kmh: Option<f32>,
    pub max_departure_delay: Option<u32>,
    pub output_format: OutputFormat,
    pub verbose: bool,
    pub output_geojson: Option<String>,
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Text,
    Json,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Text
    }
}

pub fn build_cli() -> Command {
    Command::new("bus_raptor")
        .about("Backward RAPTOR algorithm for public and private bus routing")
        .version("0.1.0")
        .author("Bus Optimization System")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("public")
                .about("Finds the best route using public transport")
                .arg(Arg::new("stops").long("stops").short('s').value_name("FILE").help("Path to stops CSV file").required(true))
                .arg(Arg::new("routes").long("routes").short('r').value_name("FILE").help("Path to routes CSV file").required(true))
                .arg(Arg::new("transfers").long("transfers").short('t').value_name("FILE").help("Path to transfers CSV file (optional)"))
                .arg(Arg::new("destination").long("destination").short('d').value_name("STOP").help("Destination stop name or ID").required(true))
                .arg(Arg::new("arrive-by").long("arrive-by").short('a').value_name("TIME").help("Target arrival time (HH:MM format)").required(true))
                .arg(Arg::new("max-transfers").long("max-transfers").value_name("NUM").help("Maximum number of transfers allowed").value_parser(clap::value_parser!(usize)).default_value("3"))
                .arg(Arg::new("walking-distance").long("walking-distance").value_name("METERS").help("Maximum walking distance for transfers (meters)").value_parser(clap::value_parser!(f32)).default_value("300"))
                .arg(Arg::new("walking-speed").long("walking-speed").value_name("KMH").help("Walking speed in km/h").value_parser(clap::value_parser!(f32)).default_value("5.0"))
                .arg(Arg::new("bus-speed").long("bus-speed").value_name("KMH").help("Average bus speed in km/h").value_parser(clap::value_parser!(f32)).default_value("30.0"))
                .arg(Arg::new("max-departure-delay").long("max-departure-delay").value_name("MINUTES").help("Maximum departure delay from target arrival time (minutes)").value_parser(clap::value_parser!(u32)).default_value("60"))
                .arg(Arg::new("output-format").long("output-format").short('f').value_name("FORMAT").help("Output format: text or json").value_parser(["text", "json"]).default_value("text"))
                .arg(Arg::new("verbose").long("verbose").short('v').help("Enable verbose output").action(ArgAction::SetTrue))
                .arg(Arg::new("output-geojson").long("output-geojson").value_name("FILE").help("Path to save GeoJSON output (optional)"))
        )
        .subcommand(
            Command::new("private")
                .about("Finds a route for a private bus with multiple stops")
                .arg(Arg::new("stops-file").long("stops-file").value_name("FILE").help("Path to stops CSV file").required(true))
                .arg(Arg::new("routes-file").long("routes-file").value_name("FILE").help("Path to routes CSV file").required(true))
                .arg(Arg::new("stop").long("stop").value_name("STOP").help("A stop to visit").required(true).action(ArgAction::Append))
                .arg(Arg::new("destination").long("destination").short('d').value_name("STOP").help("Final destination stop name").required(true))
                .arg(Arg::new("arrive-by").long("arrive-by").short('a').value_name("TIME").help("Target arrival time (HH:MM format)").required(true))
                .arg(Arg::new("output-format").long("output-format").short('f').value_name("FORMAT").help("Output format: text or json").value_parser(["text", "json"]).default_value("text"))
                .arg(Arg::new("verbose").long("verbose").short('v').help("Enable verbose output").action(ArgAction::SetTrue))
                .arg(Arg::new("output-geojson").long("output-geojson").value_name("FILE").help("Path to save GeoJSON output (optional)"))
        )
}

pub fn parse_cli() -> Result<CliCommand, Box<dyn std::error::Error>> {
    let matches = build_cli().get_matches();

    match matches.subcommand() {
        Some(("public", sub_matches)) => {
            let output_format = match sub_matches.get_one::<String>("output-format").unwrap().as_str() {
                "json" => OutputFormat::Json,
                _ => OutputFormat::Text,
            };

            let config = CliConfig {
                stops_file: sub_matches.get_one::<String>("stops").unwrap().clone(),
                routes_file: sub_matches.get_one::<String>("routes").unwrap().clone(),
                transfers_file: sub_matches.get_one::<String>("transfers").map(|s| s.clone()),
                destination: sub_matches.get_one::<String>("destination").unwrap().clone(),
                arrive_by: sub_matches.get_one::<String>("arrive-by").unwrap().clone(),
                max_transfers: sub_matches.get_one::<usize>("max-transfers").copied(),
                max_walking_distance: sub_matches.get_one::<f32>("walking-distance").copied(),
                walking_speed_kmh: sub_matches.get_one::<f32>("walking-speed").copied(),
                bus_speed_kmh: sub_matches.get_one::<f32>("bus-speed").copied(),
                max_departure_delay: sub_matches.get_one::<u32>("max-departure-delay").copied(),
                output_format,
                verbose: sub_matches.get_flag("verbose"),
                output_geojson: sub_matches.get_one::<String>("output-geojson").map(|s| s.clone()),
            };
            Ok(CliCommand::Public(config))
        }
        Some(("private", sub_matches)) => {
            let output_format = match sub_matches.get_one::<String>("output-format").unwrap().as_str() {
                "json" => OutputFormat::Json,
                _ => OutputFormat::Text,
            };

            let config = PrivateBusConfig {
                stops_file: sub_matches.get_one::<String>("stops-file").unwrap().clone(),
                routes_file: sub_matches.get_one::<String>("routes-file").unwrap().clone(),
                stops: sub_matches.get_many::<String>("stop").unwrap().map(|s| s.clone()).collect(),
                destination: sub_matches.get_one::<String>("destination").unwrap().clone(),
                arrive_by: sub_matches.get_one::<String>("arrive-by").unwrap().clone(),
                output_format,
                verbose: sub_matches.get_flag("verbose"),
                output_geojson: sub_matches.get_one::<String>("output-geojson").map(|s| s.clone()),
            };
            Ok(CliCommand::Private(config))
        }
        _ => unreachable!("Subcommand is required"),
    }
}

impl CliConfig {
    pub fn to_geo_config(&self) -> GeoConfig {
        let mut config = GeoConfig::new();
        
        if let Some(speed) = self.walking_speed_kmh {
            config = config.with_walking_speed_kmh(speed);
        }
        
        if let Some(speed) = self.bus_speed_kmh {
            config = config.with_bus_speed_kmh(speed);
        }
        
        if let Some(distance) = self.max_walking_distance {
            config = config.with_max_walking_distance_m(distance);
        }
        
        config
    }

    /// Parse destination as stop ID (if numeric) or stop name
    pub fn parse_destination(&self) -> DestinationSpec {
        if let Ok(stop_id) = self.destination.parse::<u32>() {
            DestinationSpec::Id(stop_id)
        } else {
            DestinationSpec::Name(self.destination.clone())
        }
    }

    /// Validate CLI configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate time format
        crate::raptor::journey::parse_time(&self.arrive_by)
            .map_err(|e| format!("Invalid arrive-by time: {}", e))?;

        // Check file existence
        if !std::path::Path::new(&self.stops_file).exists() {
            return Err(format!("Stops file does not exist: {}", self.stops_file));
        }

        if !std::path::Path::new(&self.routes_file).exists() {
            return Err(format!("Routes file does not exist: {}", self.routes_file));
        }

        if let Some(transfers_file) = &self.transfers_file {
            if !std::path::Path::new(transfers_file).exists() {
                return Err(format!("Transfers file does not exist: {}", transfers_file));
            }
        }

        // Validate numeric parameters
        if let Some(max_transfers) = self.max_transfers {
            if max_transfers > 10 {
                return Err("Max transfers cannot exceed 10".to_string());
            }
        }

        if let Some(walking_distance) = self.max_walking_distance {
            if walking_distance > 2000.0 {
                return Err("Walking distance cannot exceed 2000 meters".to_string());
            }
        }

        if let Some(walking_speed) = self.walking_speed_kmh {
            if walking_speed < 1.0 || walking_speed > 10.0 {
                return Err("Walking speed must be between 1 and 10 km/h".to_string());
            }
        }

        if let Some(bus_speed) = self.bus_speed_kmh {
            if bus_speed < 5.0 || bus_speed > 100.0 {
                return Err("Bus speed must be between 5 and 100 km/h".to_string());
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum DestinationSpec {
    Id(u32),
    Name(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destination_parsing() {
        let config = CliConfig {
            destination: "123".to_string(),
            stops_file: "test.csv".to_string(),
            routes_file: "test.csv".to_string(),
            transfers_file: None,
            arrive_by: "08:00".to_string(),
            max_transfers: None,
            max_walking_distance: None,
            walking_speed_kmh: None,
            bus_speed_kmh: None,
            max_departure_delay: None,
            output_format: OutputFormat::Text,
            verbose: false,
            output_geojson: None,
        };
        
        match config.parse_destination() {
            DestinationSpec::Id(id) => assert_eq!(id, 123),
            _ => panic!("Expected ID destination"),
        }
        
        let config = CliConfig {
            destination: "Tokyo Station".to_string(),
            output_geojson: None,
            ..config
        };
        
        match config.parse_destination() {
            DestinationSpec::Name(name) => assert_eq!(name, "Tokyo Station"),
            _ => panic!("Expected name destination"),
        }
    }

    #[test]
    fn test_geo_config_conversion() {
        let config = CliConfig {
            walking_speed_kmh: Some(4.0),
            bus_speed_kmh: Some(25.0),
            max_walking_distance: Some(400.0),
            stops_file: "test.csv".to_string(),
            routes_file: "test.csv".to_string(),
            transfers_file: None,
            destination: "test".to_string(),
            arrive_by: "08:00".to_string(),
            max_transfers: None,
            max_departure_delay: None,
            output_format: OutputFormat::Text,
            verbose: false,
            output_geojson: None,
        };
        
        let geo_config = config.to_geo_config();
        
        // Walking speed: 4 km/h = 4/3.6 m/s ≈ 1.11 m/s
        assert!((geo_config.walking_speed_ms - 4.0/3.6).abs() < 0.01);
        
        // Bus speed: 25 km/h = 25/3.6 m/s ≈ 6.94 m/s
        assert!((geo_config.bus_speed_ms - 25.0/3.6).abs() < 0.01);
        
        assert_eq!(geo_config.max_walking_distance_m, 400.0);
    }
}
