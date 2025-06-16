use crate::config::Config;
use chrono::{DateTime, Duration, Utc};
use rand::Rng;

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
struct DataPoint {
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
    let num_shuffles = num_entries_to_generate * 0.2 as u32;
    for _ in 0..num_shuffles {
        let (first_slot, second_slot) = pick_2_random_datapoint(duration_in_seconds);
        // update a random additive deducted from first_slot to second_slot
        let delta = rand::rng().random_range(1..datapoints[first_slot as usize].rows_to_add);
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
    Ok(())
}

fn generate_datapoints_sparse_fill(
    start_time: DateTime<Utc>,
    duration_in_seconds: i64,
    num_entries_to_generate: u32,
    datapoints: &mut Vec<DataPoint>,
) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_duration_value_and_unit() {
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
        let mut cfg = Config::new();
        cfg.set_distribution_by(Some("even".to_string()));
        cfg.set_number_of_entries(Some(10000));
        cfg.set_timestamp_format(Some("%Y-%m-%dT%H:%M:%S%.f%:z".to_string()));
        cfg.set_use_now_as_timestamp(Some(false));
        cfg.set_generation_duration(Some("10m".to_string()));
        cfg.set_start_timestamp(Some("2022-01-01T00:00:00.000+00:00".to_string()));

        // [debug]
        //println!("\n config {:?}", cfg);

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
        for _ in 0..20 {
            let result = pick_2_random_datapoint(1000);
            assert_eq!(result.0 != result.1, true);
            // [debug]
            println!("{} vs {}", result.0, result.1);
        }
    }

    #[test]
    fn test_generate_datapoints_even() {
/*
let mut cfg = Config::new();
cfg.set_distribution_by(Some("even".to_string()));
cfg.set_number_of_entries(Some(10000));
cfg.set_timestamp_format(Some("%Y-%m-%dT%H:%M:%S%.f%:z".to_string()));
cfg.set_use_now_as_timestamp(Some(false));
cfg.set_generation_duration(Some("10m".to_string()));
cfg.set_start_timestamp(Some("2022-01-01T00:00:00.000+00:00".to_string()));

cfg.set_num_entries_to_generate(Some(10000));
cfg.set_model(Some("even".to_string()));
...

*/        
    }

}
