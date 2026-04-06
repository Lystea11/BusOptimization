use bus_raptor::data::loader::load_network_from_csv;
use bus_raptor::raptor::{BackwardRaptor, PrivateBusPlanner, journey::parse_time};
use bus_raptor::cli::config::{parse_cli, CliCommand, DestinationSpec};
use bus_raptor::raptor::geojson::journey_to_geojson;
use std::process;
use std::fs::File;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let command = parse_cli()?;

    match command {
        CliCommand::Public(config) => run_public(config),
        CliCommand::Private(config) => run_private(config),
    }
}

fn run_public(config: bus_raptor::cli::config::CliConfig) -> Result<(), Box<dyn std::error::Error>> {
    config.validate()?;

    let geo_config = config.to_geo_config();
    let network = load_network_from_csv(
        &config.stops_file,
        &config.routes_file,
        config.transfers_file.as_ref(),
        Some(&geo_config),
    )?;

    let destination_id = match config.parse_destination() {
        DestinationSpec::Id(id) => id,
        DestinationSpec::Name(name) => {
            network.get_stop_by_name(&name).ok_or_else(|| format!("Stop '{}' not found", name))?
        }
    };

    let target_arrival_time = parse_time(&config.arrive_by)?;
    let max_rounds = config.max_transfers.unwrap_or(3) + 1;
    let mut raptor = BackwardRaptor::new(network.clone(), max_rounds);

    if let Some(max_delay) = config.max_departure_delay {
        raptor = raptor.with_max_departure_delay(max_delay * 60);
    }

    let journey = raptor.find_route(None, destination_id, target_arrival_time);

    if let Some(j) = journey {
        println!("{}", j.format_journey(&network));

        if let Some(geojson_path) = &config.output_geojson {
            let geojson_data = journey_to_geojson(&j, &network);
            let file = File::create(geojson_path)?;
            serde_json::to_writer_pretty(file, &geojson_data)?;
            println!("GeoJSON visualization saved to: {}", geojson_path);
        }
    } else {
        eprintln!("No route found.");
    }

    Ok(())
}

fn run_private(config: bus_raptor::cli::config::PrivateBusConfig) -> Result<(), Box<dyn std::error::Error>> {
    let network = load_network_from_csv(&config.stops_file, &config.routes_file, None, None)?;
    let stop_ids = config.stops.iter().map(|s| 
        network.get_stop_by_name(s).ok_or_else(|| format!("Stop '{}' not found", s))
    ).collect::<Result<Vec<_>, _>>()?;

    let target_arrival_time = parse_time(&config.arrive_by)?;
    let planner = PrivateBusPlanner::new(network.clone(), 5);

    let journey = planner.find_journey(stop_ids, &config.destination, target_arrival_time)?;

    println!("{}", journey.format_journey(&network));

    if let Some(geojson_path) = &config.output_geojson {
        let geojson_data = journey_to_geojson(&journey, &network);
        let file = File::create(geojson_path)?;
        serde_json::to_writer_pretty(file, &geojson_data)?;
        println!("GeoJSON visualization saved to: {}", geojson_path);
    }

    Ok(())
}