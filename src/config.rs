use std::collections::HashMap;

use getset::{Getters, Setters};
use serde::Deserialize;

use robjetives_config::{read_config_folder, BackFillable};

/// The configuration for the application.
///
/// Most of the fields are optional as the configuration system is designed to be
/// back-filled with the default values in case of missing custom values.
#[derive(Debug, Deserialize, Getters, Setters)]
pub struct Config {
    #[getset(get = "pub", set = "pub")]
    number_of_entries: Option<u32>,

    #[getset(get = "pub", set = "pub")]
    timestamp_format: Option<String>,

    #[getset(get = "pub", set = "pub")]
    use_now_as_timestamp: Option<bool>,

    #[getset(get = "pub", set = "pub")]
    generation_duration: Option<String>,

    #[getset(get = "pub", set = "pub")]
    start_timestamp: Option<String>,

    #[getset(get = "pub", set = "pub")]
    distribution_by: Option<String>,

    #[getset(get = "pub", set = "pub")]
    #[serde(rename = "exporter")]
    exporters: Option<Vec<ConfigExporter>>,
}

/// The configuration for the exporter(s) section.
#[derive(Debug, Deserialize, Getters, Setters)]
pub struct ConfigExporter {
    #[getset(get = "pub", set = "pub")]
    name: Option<String>,

    #[getset(get = "pub", set = "pub")]
    verbose: Option<bool>,

    #[getset(get = "pub", set = "pub")]
    enabled: Option<bool>,

    #[getset(get = "pub", set = "pub")]
    fields: Option<HashMap<String, String>>,
}

impl Config {
    pub fn new() -> Self {
        Config {
            number_of_entries: None,
            timestamp_format: None,
            use_now_as_timestamp: None,
            generation_duration: None,
            start_timestamp: None,
            distribution_by: None,
            exporters: None,
        }
    }
}

impl BackFillable for Config {
    fn back_fill(&mut self, from: &Self) {
        if self.number_of_entries.is_none() {
            self.set_number_of_entries(from.number_of_entries);
        }
        if self.timestamp_format.is_none() {
            self.set_timestamp_format(from.timestamp_format.clone());
        }
        if self.use_now_as_timestamp.is_none() {
            self.set_use_now_as_timestamp(from.use_now_as_timestamp);
        }
        if self.generation_duration.is_none() {
            self.set_generation_duration(from.generation_duration.clone());
        }
        if self.start_timestamp.is_none() {
            self.set_start_timestamp(from.start_timestamp.clone());
        }
        if self.distribution_by.is_none() {
            self.set_distribution_by(from.distribution_by.clone());
        }
        // not that simple; kind of merge logic instead...
        if self.exporters.is_none() {
            let mut list: Vec<ConfigExporter> = vec![];
            for e in from.exporters.as_ref().unwrap() {
                let mut exporter = ConfigExporter {
                    name: None,
                    verbose: Some(false),
                    enabled: Some(false),
                    fields: Some(HashMap::new()),
                };
                exporter.back_fill(e);
                list.push(exporter);
            }
            self.exporters = Some(list);
        } else {
            // iterate and check...
            let mut types_in_string: Vec<String> = vec![];
            let mut self_exporters = self.exporters.as_mut();
            // [original approach]
            // let mut self_exporters = self.exporters.as_ref();
            // self_exporters.unwrap().iter().for_each(|e| {
            //     types_in_string.push(e.name.as_ref().unwrap().clone());
            // });
            for e in self_exporters.as_mut().unwrap().iter_mut() {
                types_in_string.push(e.name.as_ref().unwrap().clone());
                // make sure the exporter components are non None at this point
                if e.verbose.is_none() {
                    e.set_verbose(Some(false));
                }
                if e.enabled.is_none() {
                    e.set_enabled(Some(false));
                }
                if e.fields.is_none() {
                    e.set_fields(Some(HashMap::new()));
                }
            }

            for e in from.exporters.as_ref().unwrap() {
                if !types_in_string.contains(&e.name.as_ref().unwrap().clone()) {
                    let mut exporter = ConfigExporter {
                        name: None,
                        verbose: None,
                        enabled: None,
                        fields: Some(HashMap::new()),
                        // verbose: None,
                        // enabled: None,
                        // fields: None,
                    };
                    exporter.back_fill(e);
                    self.exporters.as_mut().unwrap().push(exporter);
                }
            } // end - for(back-fill exporters looping)
        } // end - if self.exporters.is_none()
          // [debug] add robjetives_log later...
          // println!("custom: {:?}", self);
          // println!("back-fill: {:?}", from);
    }
}

impl BackFillable for ConfigExporter {
    fn back_fill(&mut self, from: &Self) {
        if self.name.is_none() {
            self.set_name(from.name.clone());
        }
        if self.verbose.is_none() {
            self.set_verbose(from.verbose);
            // well the logic is all NONE values must be gone by now
            if self.verbose().is_none() {
                self.verbose = Some(false);
            }
        }
        if self.enabled.is_none() {
            self.set_enabled(from.enabled);
            // well the logic is all NONE values must be gone by now
            if self.enabled().is_none() {
                self.enabled = Some(false);
            }
        }
        // not that easy... it is more of combining the keys within the map
        if self.fields.is_none() {
            self.set_fields(from.fields.clone());
        } else {
            // from = back-fill / default values set
            // [debug]
            // println!("from = back-fill / default values set, name = {}", self.name().as_ref().unwrap());
            if from.fields.is_none() {
                // only necessary to back-fill if the `from` struct has fields
                return;
            }
            let from_ref = from.fields.as_ref().unwrap();
            let from_keys = from_ref.keys();
            for k in from_keys {
                if self.fields.as_ref().unwrap().get(k).is_none() {
                    // more tedious but works
                    // self.fields.as_mut().unwrap().insert(k.clone(), from.fields.as_ref().unwrap().get(k).unwrap().clone());
                    self.fields
                        .as_mut()
                        .unwrap()
                        .insert(k.clone(), from_ref.get(k).unwrap().clone());
                }
            } // end - for(keys looping)
        }
    }
}

/// Load the config files and return a Config object (back-filled).
/// # Arguments
/// * `backfill_config_folder` - The path to the folder containing the backfill config files.
/// * `config_folder` - The path to the folder containing the custom config files.
/// * `file` - The name of the config file to load.
/// # Returns
/// A Config object (back-filled).
///
/// # examples
/// ```
/// use otel_broccoli::config::load_config;
///
/// let result = load_config("config/default".to_string(), "tests".to_string(), "config.toml".to_string(), "stdout_test.toml".to_string());
///
/// // should not have error...
/// assert_eq!(result.is_err(), false);
///
/// let config = result.unwrap();
/// // custom config has 1 exporter, back-fill config has 3; 1 in common is 'file' and not overwritten...
/// assert_eq!(config.exporters().as_ref().unwrap().len(), 3);
/// ```
pub fn load_config(
    backfill_config_folder: String,
    config_folder: String,
    backfill_config_file: String,
    config_file: String,
) -> Result<Config, Box<dyn std::error::Error>> {
    // load backfill config(s)
    let backfill_result = read_config_folder(
        backfill_config_folder.as_str(),
        "toml",
        backfill_config_file.as_str(),
    )?;
    // load custom config(s)
    let custom_result = read_config_folder(config_folder.as_str(), "toml", config_file.as_str())?;

    // created a mutable Config object
    let mut config: Config = toml::from_str(custom_result.get(config_file.as_str()).unwrap())?;
    let backfill_config: Config =
        toml::from_str(backfill_result.get(backfill_config_file.as_str()).unwrap())?;

    config.back_fill(&backfill_config);
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        let result = load_config(
            "config/default".to_string(),
            "tests".to_string(),
            "config.toml".to_string(),
            "stdout_test.toml".to_string(),
        );
        // should not have error
        assert_eq!(result.is_err(), false);
        let config = result.unwrap();
        // custom config has 1 exporter, back-fill config has 3; 1 in common is 'file' and not overwritten...
        assert_eq!(config.exporters().as_ref().unwrap().len(), 3);
        assert_eq!(config.number_of_entries().unwrap(), 1000);
        assert_eq!(
            config.timestamp_format().as_ref().unwrap(),
            "%Y-%m-%dT%H:%M:%S%.f%:z"
        );
        assert_eq!(config.use_now_as_timestamp().unwrap(), true);
        assert_eq!(config.generation_duration().as_ref().unwrap(), "10m");
        assert_eq!(
            config.start_timestamp().as_ref().unwrap(),
            "2022-01-01T00:00:00.000+00:00"
        );
        assert_eq!(config.distribution_by().as_ref().unwrap(), "even");
        // exporters...
        let exporters = config.exporters().as_ref().unwrap();
        let e_1_file = exporters.get(0).unwrap();
        assert_eq!(e_1_file.name().as_ref().unwrap(), "file");
        // assert_eq!(e_1_file.verbose().is_none(), true); // as optional and not back_filled in the logic
        assert_eq!(e_1_file.verbose().unwrap(), false);
        assert_eq!(e_1_file.enabled().unwrap(), true);
        assert_eq!(e_1_file.fields().as_ref().unwrap().len(), 2);
        assert_eq!(
            e_1_file.fields().as_ref().unwrap().get("filename").unwrap(),
            "stdout_test.log"
        );
        assert_eq!(
            e_1_file.fields().as_ref().unwrap().get("path").unwrap(),
            "./generated/"
        );

        let e_2_stdout = exporters.get(1).unwrap();
        assert_eq!(e_2_stdout.name().as_ref().unwrap(), "stdout");
        assert_eq!(e_2_stdout.verbose().unwrap(), false);
        assert_eq!(e_2_stdout.enabled().unwrap(), true);
        assert_eq!(e_2_stdout.fields().as_ref().unwrap().len(), 0);

        let e_3_clickhouse = exporters.get(2).unwrap();
        assert_eq!(e_3_clickhouse.name().as_ref().unwrap(), "clickhouse");
        assert_eq!(e_3_clickhouse.verbose().unwrap(), false);
        assert_eq!(e_3_clickhouse.enabled().unwrap(), false);
        assert_eq!(e_3_clickhouse.fields().as_ref().unwrap().len(), 3);
        assert_eq!(
            e_3_clickhouse
                .fields()
                .as_ref()
                .unwrap()
                .get("url")
                .unwrap(),
            "http://localhost:3125"
        );
        assert_eq!(
            e_3_clickhouse
                .fields()
                .as_ref()
                .unwrap()
                .get("user")
                .unwrap(),
            "root"
        );
        assert_eq!(
            e_3_clickhouse
                .fields()
                .as_ref()
                .unwrap()
                .get("password")
                .unwrap(),
            "password"
        );
    }
}
