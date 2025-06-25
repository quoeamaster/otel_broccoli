use crate::config::Config;
use chrono::{DateTime, Duration, Utc};
use rand::Rng;

const DEFAULT_SPARSE_FILL_ZONE_GENERATION_FACTOR: u32 = 3;

/// Generate a tuple of two DateTime values, `start_time` and `end_time`.
///
/// `start_time` is either `Utc::now()` or the value of `start_timestamp` parsed
/// with the format specified in `timestamp_format`.
///
/// `end_time` is either `start_time` if `generation_duration` is None, or
/// `start_time` plus the duration specified in `generation_duration`.
///
/// # Errors
///
/// If `start_timestamp` cannot be parsed with `timestamp_format`, an error is
/// returned. If `generation_duration` cannot be parsed, an error is returned.
///
fn generate_time_range(
    cfg: &Config,
) -> Result<(DateTime<Utc>, DateTime<Utc>), Box<dyn std::error::Error>> {
    let mut start_time = Utc::now();
    let mut end_time = Utc::now();

    if let Some(use_now) = cfg.use_now_as_timestamp() {
        // not using NOW()
        if !use_now {
            // get the `start_timestamp`
            // [lesson] if the start_timestamp is a valid dateTime format
            // start_time = cfg.start_timestamp().as_ref().unwrap().parse().unwrap();

            // [lesson] based on parsing with a format
            // start_time = DateTime::parse_from_str(
            //     cfg.start_timestamp().as_ref().unwrap(),
            //     cfg.timestamp_format().as_ref().unwrap(),
            // )
            // .unwrap()
            // .with_timezone(&Utc);

            // [lesson] might have issue on parsing if the format doesn't match with the timestamp value
            let intermediate_start_time = DateTime::parse_from_str(
                cfg.start_timestamp().as_ref().unwrap(),
                cfg.timestamp_format().as_ref().unwrap(),
            );
            if intermediate_start_time.is_err() {
                return Err(format!(
                    "failed to parse start_timestamp [{}] with format [{}]: {}",
                    cfg.start_timestamp().as_ref().unwrap(),
                    cfg.timestamp_format().as_ref().unwrap(),
                    intermediate_start_time.err().unwrap()
                )
                .into());
            }
            start_time = intermediate_start_time.unwrap().with_timezone(&Utc);
            // [lesson] DateTime has implemented the Copy trait
            // end_time = start_time.clone();
            end_time = start_time;
        }
    }
    // update the end_time with the value = generation_duration
    if let Some(generation_duration) = cfg.generation_duration() {
        // throw the error to upper stack OR get the duration value
        let value_and_unit = parse_time_duration(generation_duration.clone())?;
        end_time = start_time + value_and_unit;
    }

    Ok((start_time, end_time))
}

/// parse the time duration value and unit from the given string value.
fn parse_time_duration_value_and_unit(value: String) -> Option<(i64, String)> {
    // find out which index is a non-numeric value
    let idx = value.find(|c: char| !c.is_ascii_digit())?;
    let (num, unit) = value.split_at(idx);
    let num: i64 = num.parse::<i64>().ok()?;

    Some((num, unit.to_string()))
}

/// parse the time duration based on the given string value.
/// For non supported value (invalid format etc) would return zero duration.
fn parse_time_duration(value: String) -> Result<Duration, Box<dyn std::error::Error>> {
    let parsed_value_and_unit = parse_time_duration_value_and_unit(value);
    if parsed_value_and_unit.is_none() {
        return Err("failed to parse time duration value and unit"
            .to_string()
            .into());
    }

    let (num, unit) = parsed_value_and_unit.unwrap();
    match unit.as_str() {
        "s" => Ok(Duration::seconds(num)),
        "m" => Ok(Duration::minutes(num)),
        "h" => Ok(Duration::hours(num)),
        "d" => Ok(Duration::days(num)),
        _ => {
            // anything else is not supported and return zero duration...
            // Err("invalid time duration unit".to_string().into()),
            Ok(Duration::zero())
        }
    } // end - match
}

/// struct to hold the timestamp and the number of rows to add - acts as a DataPoint in the distribution.
#[derive(Debug)]
pub struct DataPoint {
    timestamp: DateTime<Utc>,
    rows_to_add: i16,
}

pub fn generate_datapoints(cfg: &Config) -> Result<Vec<DataPoint>, Box<dyn std::error::Error>> {
    let mut datapoints: Vec<DataPoint> = Vec::new();
    let (start_time, _) = generate_time_range(cfg)?;

    // [lesson] also works ... cfg.generation_duration().as_ref().unwrap().clone()
    let duration = parse_time_duration(cfg.generation_duration().as_deref().unwrap().to_string())?;
    // duration in seconds is the unit of time for generating datapoints.
    // Seconds granularity works in this case as though in production, events are created at microseconds or milliseconds level;
    // however for graph plotting etc, the datapoints are usually re-grouped in a less granular unit such as seconds, minutes or days
    // and thus would not make much difference to have a microsecond granularity or not.
    //
    // PS. you might view this as a limitation of the implementation.
    let duration_in_seconds = duration.num_seconds();

    let num_entries_to_generate = cfg.number_of_entries().as_ref().unwrap().clone();
    let model = cfg.distribution_by().as_deref().unwrap().to_lowercase();
    match model.as_str() {
        "even" => generate_datapoints_even(
            start_time,
            duration_in_seconds,
            num_entries_to_generate,
            &mut datapoints,
        )?,
        "early_fill" => generate_datapoints_early_fill(
            start_time,
            duration_in_seconds,
            num_entries_to_generate,
            &mut datapoints,
        )?,
        "sparse_fill" => generate_datapoints_sparse_fill(
            start_time,
            duration_in_seconds,
            num_entries_to_generate,
            &mut datapoints,
        )?,
        _ => {
            return Err(format!("unknown distribution model [{}]", model)
                .to_string()
                .into())
        }
    }
    Ok(datapoints)
}

fn generate_datapoints_even(
    start_time: DateTime<Utc>,
    duration_in_seconds: i64,
    num_entries_to_generate: u32,
    datapoints: &mut Vec<DataPoint>,
) -> Result<(), Box<dyn std::error::Error>> {
    // approximately per datapoint interval should generate how many rows?
    let per_datapoint_entries_to_generate = num_entries_to_generate as i64 / duration_in_seconds;

    // first fill
    let mut sum = 0;
    let last_datapoint_index = duration_in_seconds - 1;
    for i in 0..duration_in_seconds {
        if i != last_datapoint_index {
            datapoints.push(DataPoint {
                timestamp: start_time + Duration::seconds(i),
                rows_to_add: per_datapoint_entries_to_generate as i16,
            });
            sum += per_datapoint_entries_to_generate;
        } else {
            datapoints.push(DataPoint {
                timestamp: start_time + Duration::seconds(i),
                rows_to_add: num_entries_to_generate as i16 - sum as i16,
            });
        }
    } // end - for duration_in_seconds loop

    // second fill (random pick and assign)
    // rounds 2/10 of the num_of_entries_to_generate, make sure a randomness is introduced in the distribution set.
    let num_shuffles = (num_entries_to_generate as f32 * 0.2) as u32;
    for _ in 0..num_shuffles {
        let (first_slot, second_slot) = pick_2_random_datapoint(duration_in_seconds);
        // update a random additive deducted from first_slot to second_slot
        let first_slot_row_to_add = datapoints[first_slot as usize].rows_to_add;
        tracing::trace!(
            "first_slot={} vs second_slot={} - first_slot_in_usize {}, rows_to_add {}",
            first_slot,
            second_slot,
            first_slot as usize,
            first_slot_row_to_add
        );
        if first_slot_row_to_add == 1 {
            continue;
        }
        let delta = rand::rng().random_range(1..first_slot_row_to_add);
        datapoints[first_slot as usize].rows_to_add -= delta;
        datapoints[second_slot as usize].rows_to_add += delta;
    }
    Ok(())
}

fn pick_2_random_datapoint(slots_length: i64) -> (i64, i64) {
    // slots_length = duration_in_seconds
    let first_slot = rand::rng().random_range(0..slots_length);
    let mut second_slot = rand::rng().random_range(0..slots_length);

    loop {
        if second_slot != first_slot {
            break;
        }
        second_slot = rand::rng().random_range(0..slots_length);
    }
    return (first_slot, second_slot);
}

fn generate_datapoints_early_fill(
    start_time: DateTime<Utc>,
    duration_in_seconds: i64,
    num_entries_to_generate: u32,
    datapoints: &mut Vec<DataPoint>,
) -> Result<(), Box<dyn std::error::Error>> {
    // loop through the slots
    // assign a random rows_to_add value to the given slot
    //  (remember the actual ceiling is the num_entries_to_generate; so a logical ceiling would be num_entries_to_generate * 1% per slot's rows_to_add')
    // once the accumulated rows_to_add is greater than or equals to num_entries_to_generate, augmentation done and can't exit the allocation.

    let logical_ceiling = (num_entries_to_generate as f32 * 0.01) as u32;
    let logical_floor: u32 = 1;

    let mut sum = 0;
    // [deprecated] used to create `empty` datapoints, but not make sense for most use case, hence simply drop it.
    // let mut done_allocation = false;
    // let mut early_log = false;
    for i in 0..duration_in_seconds {
        let mut rows_to_add = rand::rng().random_range(logical_floor..=logical_ceiling);
        // guard check
        if sum + rows_to_add > num_entries_to_generate {
            rows_to_add = num_entries_to_generate - sum;
            sum = num_entries_to_generate;
        } else {
            sum += rows_to_add;
        }
        // push a datapoint
        // even though empty rows_to_add, must still have a datapoint
        datapoints.push(DataPoint {
            timestamp: start_time + Duration::seconds(i),
            rows_to_add: rows_to_add as i16,
        });
        if sum == num_entries_to_generate {
            // [log]
            tracing::info!(
                message = format!(
                    "{} of distribution all early filled at idx {}, saved {} rows to generate",
                    num_entries_to_generate,
                    i,
                    duration_in_seconds - i
                ),
                module = "augmentation"
            );
            break;
        } // end - if (sum == num_entries_to_generate)
    }
    Ok(())
}

fn generate_datapoints_sparse_fill(
    start_time: DateTime<Utc>,
    duration_in_seconds: i64,
    num_entries_to_generate: u32,
    datapoints: &mut Vec<DataPoint>,
) -> Result<(), Box<dyn std::error::Error>> {
    // create a random number of `zones`;
    //   each zone would be allocated a number of datapoints to be generated. (also another random value based on num_entries_to_generate)
    // there would be a random gap between the `zones`; could be 0 - adjacent with the previous zone. Or could be a random number of seconds (etc)
    //   however, the last zone's outer boundary must be touching the the last datapoint's timestamp.
    //   hence the logic would make sense in this way
    //   - calculate the first zone's boundaries
    //   - calculate the last zone's boundaries
    //   - the residual boundary would be shared with the remaining zone(s).
    //   - each zone would be allocated a random rows_to_add value based on num_entries_to_generate.

    let num_of_zone = rand::rng().random_range(3..=6);
    let zone_allocation_ceiling = num_entries_to_generate / num_of_zone;
    let mut zone_allocations: Vec<u32> = vec![];

    // first fill for zone_allocations
    let mut sum = 0;
    for i in 0..num_of_zone {
        if i == num_of_zone - 1 {
            zone_allocations.push(num_entries_to_generate - sum);
            break;
        } else {
            zone_allocations.push(zone_allocation_ceiling);
        }
        sum += zone_allocation_ceiling;
    }
    // shuffling
    // - based on num_of_zone * 5 times of shuffle
    for _ in 0..num_of_zone * 5 {
        let (first_slot, second_slot) = pick_2_random_datapoint(num_of_zone as i64);
        // generate a random delta
        let upper_bound = zone_allocations[first_slot as usize];
        if upper_bound < 2 {
            continue;
        }
        let delta = rand::rng().random_range(1..upper_bound);

        zone_allocations[first_slot as usize] -= delta;
        zone_allocations[second_slot as usize] += delta;
    }
    // [log]
    tracing::debug!(
        message = format!(
            "number of zones {} for sparse-fill after shuffle, ceilings per zone: {:?}",
            num_of_zone, zone_allocations
        ),
        module = "augmentation"
    );

    // logic of slots...
    // - num_of_zones = 6 -> slots available = num_of_zones * 6 = 36;
    // - each slots boundary is the result of an even value of the duration_in_seconds; ie. duration_in_seconds / num_of_zone_slots (36 in this case);
    // - now each zone would pick 1 or more slots; should say a random slot occupancy per zone is calculated.
    // - But worst case is per zone would have occupied at least 1 slot.
    // - which means per zone would need to calculate the following
    //   - no. of zone slots to occupy
    //   - find a section of the zone slots that could fill up this value (worst case, round back to 1 single slot if no availability)
    //
    // a very simple implementation
    // - first round of allocation is - zone's number of slots to occupy (1..=3); sum up should not exceed the total number of zone slots (36 in this case)
    //   - during this round, the to-be-rows-add value would be allocated based on num_entries_to_generate.
    // - second round of allocation is - calculate the zone's gap (1..=3); hence gap + zone boundary should at most meet the the duration_in_seconds value
    //   - during this round the allocation of zone's to-be-rows-add would be done and spread through the zone's boundary.
    //
    // so 36 zone slots... each should have a data-structure declaring what should the zone slot's operation be
    // - do nothing since it is a Gap
    // - allocate the rows_to_add value evenly

    // next -> zone slots and how to divide it (duration_in_seconds / (num_of_zone * 6))
    let zone_slots = generate_sparse_fill_zone_and_boundaries(
        &zone_allocations,
        DEFAULT_SPARSE_FILL_ZONE_GENERATION_FACTOR,
        start_time,
        duration_in_seconds,
    );
    // loop through; if DataZone.num_rows_to_add > 0; call fn to add back DataPoint(s)
    // hence the output would be a bunch of datapoints in which there would be gap(s) in the timestamp
    // (since there are zones without data being generated)
    for zone in zone_slots {
        if zone.num_rows_to_add > 0 {
            let mut updated_datapoints = generate_sparse_fill_zone_datapoints(&zone);
            datapoints.append(&mut updated_datapoints);
        }
    }
    Ok(())
}

fn generate_sparse_fill_zone_and_boundaries(
    data_zones_to_be_generated: &Vec<u32>,
    generation_factor: u32,
    start_time: DateTime<Utc>,
    duration_in_seconds: i64,
) -> Vec<DataZone> {
    // eg. generation_factor = 6
    // num_of_data_zones = data_zones_to_be_generated.len() = 5
    // size of vec would be 6*5 = 30; out 5 would be occupied
    let data_zones_len = generation_factor as usize * data_zones_to_be_generated.len();
    let mut data_zones: Vec<DataZone> = vec![
        DataZone::new();
        // DataZone {
        //     start_time: Utc::now(),
        //     end_time: Utc::now(),
        //     num_rows_to_add: 0,
        // };
        data_zones_len
    ];
    // first iteration; fill up start_time, end_time
    let mut zone_idx = 0;
    let zone_span = duration_in_seconds / data_zones_len as i64;
    for zone in data_zones.iter_mut() {
        zone.start_time = start_time + Duration::seconds(zone_span * zone_idx as i64);

        if zone_idx == data_zones_len - 1 {
            zone.end_time = start_time + Duration::seconds(duration_in_seconds);
        } else {
            zone.end_time = zone.start_time + Duration::seconds(zone_span - 1);
        }
        zone_idx += 1;
    }
    // pick which zone to fill and which not
    for zone in data_zones_to_be_generated.iter() {
        loop {
            let idx = rand::rng().random_range(0..data_zones.len());
            if data_zones[idx].num_rows_to_add == 0 {
                data_zones[idx].num_rows_to_add = *zone;
                break;
            }
        }
    }
    data_zones
}

#[derive(Clone, Debug)]
struct DataZone {
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    num_rows_to_add: u32,
}

impl DataZone {
    fn new() -> DataZone {
        DataZone {
            start_time: Utc::now(),
            end_time: Utc::now(),
            num_rows_to_add: 0,
        }
    }
}

fn generate_sparse_fill_zone_datapoints(data_zone: &DataZone) -> Vec<DataPoint> {
    let mut data_points = Vec::new();
    // calculate the duration
    let duration = data_zone.end_time.timestamp() - data_zone.start_time.timestamp();
    // [trace] make it a trace after dev completed
    tracing::debug!(
        module = "augmentation",
        message = format!(
            "data_zone duration: {} seconds",
            data_zone.end_time.timestamp() - data_zone.start_time.timestamp()
        )
    );
    let mut rows_to_add_per_second = data_zone.num_rows_to_add / duration as u32;
    let mut sum = 0;
    // first fill with equal num of rows
    for i in 0..duration {
        // last entry
        if i == duration - 1 {
            rows_to_add_per_second = data_zone.num_rows_to_add - sum;
        }
        data_points.push(DataPoint {
            timestamp: data_zone.start_time + Duration::seconds(i),
            rows_to_add: rows_to_add_per_second as i16,
        });
        sum += rows_to_add_per_second;
    }
    // second fill is shuffling by a factor of duration * 3;
    for _ in 0..duration * 3 {
        let (idx_1, idx_2) = pick_2_random_datapoint(data_points.len() as i64);
        let rows_available = data_points[idx_1 as usize].rows_to_add;
        if rows_available < 2 {
            continue;
        }
        let delta = rand::rng().random_range(1..rows_available);

        data_points[idx_1 as usize].rows_to_add -= delta;
        data_points[idx_2 as usize].rows_to_add += delta;
    }
    data_points
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_init;

    #[test]
    fn test_parse_time_duration_value_and_unit() {
        // init loggers
        app_init("./config/default/loggers.toml".to_string()).unwrap();

        let result = parse_time_duration_value_and_unit("10m".to_string());
        assert_eq!(result.is_some(), true);
        assert_eq!(result.as_ref().unwrap().0, 10);
        assert_eq!(result.as_ref().unwrap().1, "m".to_string());

        // for invalid values... it still parse as is...
        let result = parse_time_duration_value_and_unit("10m3d".to_string());
        assert_eq!(result.is_some(), true);
        assert_eq!(result.as_ref().unwrap().0, 10);
        assert_eq!(result.as_ref().unwrap().1, "m3d".to_string());

        // totally non-parsable value will yield NONE
        let result = parse_time_duration_value_and_unit("m10".to_string());
        assert_eq!(result.is_some(), false);
        assert_eq!(result.is_none(), true);
    }

    #[test]
    fn test_parse_time_duration() {
        // init loggers
        app_init("./config/default/loggers.toml".to_string()).unwrap();

        let result = parse_time_duration("10m".to_string());
        assert_eq!(result.is_ok(), true);
        assert_eq!(
            result.as_ref().unwrap().num_nanoseconds().unwrap(),
            Duration::minutes(10).num_nanoseconds().unwrap()
        );

        let result = parse_time_duration("10s".to_string());
        assert_eq!(result.is_ok(), true);
        assert_eq!(
            result.as_ref().unwrap().num_nanoseconds().unwrap(),
            Duration::seconds(10).num_nanoseconds().unwrap()
        );

        // totally not parsable value
        let result = parse_time_duration("f10m".to_string());
        assert_eq!(result.is_ok(), false);
        assert_eq!(
            result.err().unwrap().to_string(),
            "failed to parse time duration value and unit"
        );
    }

    // generate_time_range()
    // create an artifial Config struct with combos to test around
    #[test]
    fn test_generate_time_range() {
        // init loggers
        app_init("./config/default/loggers.toml".to_string()).unwrap();

        let mut cfg = Config::new();
        cfg.set_distribution_by(Some("even".to_string()));
        cfg.set_number_of_entries(Some(10000));
        cfg.set_timestamp_format(Some("%Y-%m-%dT%H:%M:%S%.f%:z".to_string()));
        cfg.set_use_now_as_timestamp(Some(false));
        cfg.set_generation_duration(Some("10m".to_string()));
        cfg.set_start_timestamp(Some("2022-01-01T00:00:00.000+00:00".to_string()));

        tracing::trace!("config: {:#?}", cfg);

        // [case][01] not using NOW(), provide a valid timestamp_format + start_timestamp
        let result = generate_time_range(&cfg);
        if result.is_err() {
            assert_eq!(result.err().unwrap().to_string(), "whay?");
            return;
        }
        // [lesson] work... but hard to understood the nanoseconds value for comparison
        //assert_eq!(result.as_ref().unwrap().0, 1640995200000); // 2022-01-01T00:00:00.000Z
        //assert_eq!(result.as_ref().unwrap().1, 1640995201000); // 2022-01-01T00:00:10.000Z
        let mut start_time_test: DateTime<Utc> = "2022-01-01T00:00:00.000Z".parse().unwrap();
        let mut end_time_test: DateTime<Utc> = start_time_test + Duration::minutes(10);
        assert_eq!(
            result.as_ref().unwrap().0.timestamp_millis(),
            start_time_test.timestamp_millis()
        );
        assert_eq!(
            result.as_ref().unwrap().1.timestamp_millis(),
            end_time_test.timestamp_millis()
        );

        // [case][02] not using NOW(), provide a in-valid timestamp_format + start_timestamp
        cfg.set_timestamp_format(Some("invalid-simply".to_string()));
        let result = generate_time_range(&cfg);
        if result.is_err() {
            // failed to parse start_timestamp [2022-01-01T00:00:00.000+00:00] with format [invalid-simply]: input contains invalid characters
            assert_eq!(
                result
                    .err()
                    .unwrap()
                    .to_string()
                    .find("input contains invalid characters")
                    .is_some(),
                true
            );
        }

        // [case][03] not using NOW(), provide a valid timestamp_format + in-Valid start_timestamp
        cfg.set_timestamp_format(Some("%Y-%m-%dT%H:%M:%S%.f%:z".to_string()));
        cfg.set_start_timestamp(Some("invalid-timestamp-value".to_string()));
        let result = generate_time_range(&cfg);
        if result.is_err() {
            // failed to parse start_timestamp [2022-01-01T00:00:00.000+00:00] with format [invalid-simply]: input contains invalid characters
            assert_eq!(
                result
                    .err()
                    .unwrap()
                    .to_string()
                    .find("input contains invalid characters")
                    .is_some(),
                true
            );
        }

        // [case][04] using NOW(), compare with current time
        // (discrepancies should be within 1 seconds, the start_time_test should be roughly 1 sec after the acutal call)
        //cfg.set_timestamp_format(Some("%Y-%m-%dT%H:%M:%S%.f%:z".to_string()));
        //cfg.set_start_timestamp(Some("2022-01-01T00:00:00.000+00:00".to_string()));
        cfg.set_use_now_as_timestamp(Some(true));
        start_time_test = Utc::now();
        end_time_test = start_time_test + Duration::minutes(10);
        let result = generate_time_range(&cfg);
        if result.is_err() {
            assert_eq!(result.err().unwrap().to_string(), "huh?");
            return;
        }
        let start_diff =
            result.as_ref().unwrap().0.timestamp_millis() - start_time_test.timestamp_millis();
        let end_diff =
            result.as_ref().unwrap().1.timestamp_millis() - end_time_test.timestamp_millis();
        assert_eq!(start_diff >= 0 && start_diff <= 1000, true);
        assert_eq!(end_diff >= 0 && end_diff <= 1000, true);
    }

    #[test]
    fn test_pick_2_random_datapoint() {
        // init loggers
        app_init("./config/default/loggers.toml".to_string()).unwrap();

        for _ in 0..20 {
            let result = pick_2_random_datapoint(1000);
            assert_eq!(result.0 != result.1, true);

            tracing::trace!("{} vs {}", result.0, result.1);
        }
    }

    #[test]
    fn test_generate_datapoints_even() {
        // init loggers
        app_init("./config/default/loggers.toml".to_string()).unwrap();

        let mut cfg = Config::new();
        cfg.set_distribution_by(Some("even".to_string()));
        cfg.set_number_of_entries(Some(10000));
        cfg.set_timestamp_format(Some("%Y-%m-%dT%H:%M:%S%.f%:z".to_string()));
        cfg.set_use_now_as_timestamp(Some(false));
        cfg.set_generation_duration(Some("10m".to_string()));
        cfg.set_start_timestamp(Some("2022-01-01T00:00:00.000+00:00".to_string()));

        let result = generate_datapoints(&cfg);
        assert_eq!(result.is_err(), false);
        tracing::trace!("{:?}", result.as_ref().unwrap());

        let mut sum = 0;
        let mut histogram = String::new();
        let datapoints = result.as_ref().unwrap();
        for datapoint in datapoints {
            sum += datapoint.rows_to_add;
            // [debug]
            // [graph - histogram]
            histogram.push_str(format!("timestamp: {} | ", datapoint.timestamp).as_str());
            for _ in 0..datapoint.rows_to_add {
                histogram.push_str(".");
            }
            histogram.push_str("\n");
        }
        tracing::info!("\n{}", histogram);
        assert_eq!(sum as u32 == cfg.number_of_entries().unwrap(), true);
    }

    #[test]
    fn test_generate_datapoints_early_fill() {
        // init loggers
        app_init("./config/default/loggers.toml".to_string()).unwrap();

        let mut cfg = Config::new();
        cfg.set_distribution_by(Some("early_fill".to_string()));
        cfg.set_number_of_entries(Some(10000));
        cfg.set_timestamp_format(Some("%Y-%m-%dT%H:%M:%S%.f%:z".to_string()));
        cfg.set_use_now_as_timestamp(Some(false));
        cfg.set_generation_duration(Some("10m".to_string()));
        cfg.set_start_timestamp(Some("2022-01-01T00:00:00.000+00:00".to_string()));

        let result = generate_datapoints(&cfg);
        assert_eq!(result.is_err(), false);
        tracing::trace!("{:?}", result.as_ref().unwrap());

        let mut sum = 0;
        let mut histogram = String::new();
        let datapoints = result.as_ref().unwrap();
        for datapoint in datapoints {
            sum += datapoint.rows_to_add;
            // [debug]
            // [graph - histogram]
            histogram.push_str(format!("timestamp: {} | ", datapoint.timestamp).as_str());
            for _ in 0..datapoint.rows_to_add {
                histogram.push_str(".");
            }
            histogram.push_str("\n");
        }
        tracing::info!("\n{}", histogram);
        tracing::info!(
            "sum: {} vs num_entries: {}",
            sum,
            cfg.number_of_entries().unwrap()
        );
        assert_eq!(sum as u32 == cfg.number_of_entries().unwrap(), true);
    }

    #[test]
    fn test_generate_datapoints_sparse_fill() {
        // init loggers
        app_init("./config/default/loggers.toml".to_string()).unwrap();

        let mut cfg = Config::new();
        cfg.set_distribution_by(Some("sparse_fill".to_string()));
        cfg.set_number_of_entries(Some(10000));
        cfg.set_timestamp_format(Some("%Y-%m-%dT%H:%M:%S%.f%:z".to_string()));
        cfg.set_use_now_as_timestamp(Some(false));
        cfg.set_generation_duration(Some("10m".to_string()));
        cfg.set_start_timestamp(Some("2022-01-01T00:00:00.000+00:00".to_string()));

        let result = generate_datapoints(&cfg);
        assert_eq!(result.is_err(), false);
        tracing::trace!("{:?}", result.as_ref().unwrap());

        let mut sum = 0;
        let mut histogram = String::new();
        let datapoints = result.as_ref().unwrap();
        for datapoint in datapoints {
            sum += datapoint.rows_to_add;
            // [debug]
            // [graph - histogram]
            histogram.push_str(format!("timestamp: {} | ", datapoint.timestamp).as_str());
            for _ in 0..datapoint.rows_to_add {
                histogram.push_str(".");
            }
            histogram.push_str("\n");
        }
        tracing::info!("\n{}", histogram);
        tracing::info!(
            "sum: {} vs num_entries: {}",
            sum,
            cfg.number_of_entries().unwrap()
        );
        assert_eq!(sum as u32 == cfg.number_of_entries().unwrap(), true);
    }

    #[test]
    fn test_generate_sparse_fill_zone_and_boundaries() {
        // init loggers
        app_init("./config/default/loggers.toml".to_string()).unwrap();

        // table test(s) / parameterized test(s)
        // parameters
        // 1. data_zones_to_be_generated: &Vec<u32>,
        // 2. generation_factor: u32,
        // 3. start_time: DateTime<Utc>,
        // 4. duration_in_seconds: i64
        // 5. expect error message: str
        // 6. expect number of data zones: u32 => (1.len() x 2.)
        // 7. sum of vec![] in 1.
        let test_cases = vec![
            (
                vec![100, 190, 100, 60],
                6,
                Utc::now(),
                10 * 60,
                4 * 6,
                100 + 190 + 100 + 60,
            ),
            (
                vec![30, 80, 120],
                8,
                Utc::now(),
                8 * 60,
                3 * 8,
                30 + 80 + 120,
            ),
            (
                vec![100, 190, 100, 60],
                3,
                Utc::now(),
                10 * 60,
                4 * 3,
                100 + 190 + 100 + 60,
            ),
        ];
        // iterate the test_cases
        for (
            data_zones_to_be_generated,
            generation_factor,
            start_time,
            duration_in_seconds,
            expect_number_of_data_zones,
            expect_sum,
        ) in test_cases
        {
            let data_zones = generate_sparse_fill_zone_and_boundaries(
                &data_zones_to_be_generated,
                generation_factor,
                start_time,
                duration_in_seconds,
            );
            assert_eq!(
                data_zones.len() as u32,
                expect_number_of_data_zones,
                "expect {} zones created with {} rows altogether",
                expect_number_of_data_zones,
                expect_sum
            );
            let mut sum = 0;
            for data_zone in data_zones.clone() {
                sum += data_zone.num_rows_to_add;
            }
            assert_eq!(
                sum as u32, expect_sum,
                "expect {} zones created with {} rows altogether",
                expect_number_of_data_zones, expect_sum
            );
            // all is good, trace a message
            tracing::info!(
                "{} zones created with {} rows altogether, distribution: {:?}",
                expect_number_of_data_zones,
                expect_sum,
                data_zones
            );
        }
    }
}
