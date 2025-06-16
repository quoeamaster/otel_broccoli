use crate::config::Config;
use chrono::{DateTime, Duration, Utc};

pub fn generate_time_range(
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
            end_time = start_time.clone();
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

fn parse_time_duration_value_and_unit(value: String) -> Option<(i64, String)> {
    // find out which index is a non-numeric value
    let idx = value.find(|c: char| !c.is_ascii_digit())?;
    let (num, unit) = value.split_at(idx);
    let num: i64 = num.parse::<i64>().ok()?;

    Some((num, unit.to_string()))
}

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
}
