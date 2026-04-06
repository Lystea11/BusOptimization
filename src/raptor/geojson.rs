use crate::data::structures::{Network, Stop, Journey, JourneyLeg};
use geojson::{Feature, FeatureCollection, Geometry, JsonObject, Value};

pub fn journey_to_geojson(journey: &Journey, network: &Network) -> FeatureCollection {
    let mut features = Vec::new();
    let mut line_string_coords = Vec::new();

    if journey.legs.is_empty() {
        return FeatureCollection {
            bbox: None,
            features,
            foreign_members: None,
        };
    }

    // Add the first stop
    let first_leg = &journey.legs[0];
    let start_stop_id = match first_leg {
        JourneyLeg::Bus { from_stop, .. } => *from_stop,
        JourneyLeg::Walk { from_stop, .. } => *from_stop,
    };
    if let Some(start_stop) = network.get_stop(start_stop_id) {
        line_string_coords.push(vec![start_stop.lon as f64, start_stop.lat as f64]);
        features.push(stop_to_feature(start_stop, "start"));
    }

    // Iterate over legs to build the LineString and intermediate stop points
    for leg in &journey.legs {
        let (to_stop_id, leg_type) = match leg {
            JourneyLeg::Bus { to_stop, .. } => (*to_stop, "intermediate"),
            JourneyLeg::Walk { to_stop, .. } => (*to_stop, "intermediate"),
        };

        if let Some(stop) = network.get_stop(to_stop_id) {
            line_string_coords.push(vec![stop.lon as f64, stop.lat as f64]);
            features.push(stop_to_feature(stop, leg_type));
        }
    }
    
    // Change the last stop type to "destination"
    if let Some(last_feature) = features.last_mut() {
        if let Some(properties) = last_feature.properties.as_mut() {
            properties.insert("type".to_string(), "destination".into());
        }
    }


    // Create LineString feature
    let linestring = Geometry::new(Value::LineString(line_string_coords));
    let mut properties = JsonObject::new();
    properties.insert("type".to_string(), "route".into());
    features.insert(
        0,
        Feature {
            bbox: None,
            geometry: Some(linestring),
            id: None,
            properties: Some(properties),
            foreign_members: None,
        },
    );

    FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    }
}

fn stop_to_feature(stop: &Stop, stop_type: &str) -> Feature {
    let point = Geometry::new(Value::Point(vec![stop.lon as f64, stop.lat as f64]));
    let mut properties = JsonObject::new();
    properties.insert("name".to_string(), stop.name.clone().into());
    properties.insert("type".to_string(), stop_type.to_string().into());
    Feature {
        bbox: None,
        geometry: Some(point),
        id: None,
        properties: Some(properties),
        foreign_members: None,
    }
}