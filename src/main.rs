use image::{DynamicImage, ImageBuffer};

use lazy_static::lazy_static;
use rand::distributions::Uniform;
use rand::prelude::SliceRandom;
use rand::Rng;
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::collections::HashSet;
use std::error::Error;
use std::fs::{read_dir, File};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::Path;
use std::time::Instant;
use std::{fmt, fs};
use strsim::levenshtein;
use walkdir::WalkDir;

lazy_static! {
    static ref RE_WEIGHT: Regex = Regex::new(r"#\d*").unwrap();
    static ref RE_FILENAME: Regex = Regex::new(r"^(.*?)(?:#(\d+))?\..*$").unwrap();
    static ref RE_PATH: Regex = Regex::new(r"#\d+|\.\w+$").unwrap();
    static ref ALLOWED_EXTENSION: &'static str = "png";
}

#[derive(Debug, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "camelCase")]
struct Image {
    width: u32,
    height: u32,
}
#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
struct Combination {
    layer: String,
    value: String,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
struct ForcedCombo {
    layer: Layer,
    value: String,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
#[allow(non_snake_case)]
enum Layer {
    Simple(String),
    Complex { mainLayer: String, subLayer: String },
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
struct ForcedCombinations {
    combo: Vec<ForcedCombo>,
    percentage: u8,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
struct Config {
    metadata: HashMap<String, Value>,
    image: Image,
    total_supply: u32,
    base_path: String,
    output_path: String,
    image_url: String,
    layer_folders: Vec<String>,
    skipped_traits: Option<Vec<String>>,
    forced_combinations: Vec<ForcedCombinations>,
}
#[derive(Serialize, Clone)]
struct Attribute {
    trait_type: String,
    value: String,
    weight: f64,

}

#[derive(Debug)]
pub enum CustomError {
    GetEntriesByPath(String),
    InvalidTrait(String),
    InvalidTotalSupply(u64, u64),
    TotalPercentageExceeded(String),
    InvalidImageExtension(String),
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CustomError::GetEntriesByPath(ref path) => {
                write!(f, "Failed to retrieve entries by path folder: {}", path)
            }
            CustomError::InvalidTrait(ref msg) => write!(f, "Invalid trait config: {}", msg),
            CustomError::InvalidTotalSupply(expected, actual) => write!(
                f,
                "Invalid total supply. Expected: {}. Actual: {}.",
                expected, actual
            ),
            CustomError::TotalPercentageExceeded(ref msg) => {
                write!(f, "{}", msg)
            }
            CustomError::InvalidImageExtension(ref msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

impl PartialEq for CustomError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CustomError::GetEntriesByPath(msg1), CustomError::GetEntriesByPath(msg2)) => {
                msg1 == msg2
            }
            (CustomError::InvalidTrait(msg1), CustomError::InvalidTrait(msg2)) => msg1 == msg2,
            (
                CustomError::InvalidTotalSupply(expected1, actual1),
                CustomError::InvalidTotalSupply(expected2, actual2),
            ) => expected1 == expected2 && actual1 == actual2,
            _ => false,
        }
    }
}

impl Error for CustomError {}

fn get_entries_by_path_dir(path: String) -> Result<Vec<String>, CustomError> {
    let mut entries: Vec<String> = Vec::new();
    let base_path_dir = read_dir(path.clone());
    match base_path_dir {
        Ok(dir_entries) => {
            for entry in dir_entries {
                if let Ok(entry) = entry {
                    entries.push(entry.path().display().to_string());
                }
            }
            Ok(entries)
        }
        Err(_) => Err(CustomError::GetEntriesByPath(path)),
    }
}

fn compare_and_verify_traits<T: PartialEq + Ord + Clone>(
    mut traits_by_path: Vec<String>,
    mut traits_by_config: Vec<String>,
    base_path: &str,
) -> Result<Vec<String>, CustomError> {
    if traits_by_path.len() != traits_by_config.len() {
        return Err(CustomError::InvalidTrait(format!(
            "[traits_by_path: {} traits_by_config: {}]",
            traits_by_path.len(),
            traits_by_config.len()
        )));
    }

    let mut treated = traits_by_path
        .iter()
        .map(|s| s.split('#').next().unwrap_or("").to_string())
        .collect::<Vec<String>>();

    treated.sort();
    let unordered_traits_by_config = traits_by_config.clone();
    traits_by_config.sort();

    if treated != traits_by_config {
        return Err(CustomError::InvalidTrait(
            "[traits_by_path and traits_by_config are different]".to_string(),
        ));
    }

    traits_by_path.sort_by_cached_key(|item| {
        let mut best_score = usize::MAX;
        let mut best_index = 0;

        for (index, first_item) in unordered_traits_by_config.iter().enumerate() {
            let score = levenshtein(first_item, &item.trim_start_matches(base_path));
            if score < best_score {
                best_score = score;
                best_index = index;
            }
        }

        best_index
    });

    Ok(traits_by_path)
}

fn calculate_weights_and_total(layer: &[String]) -> (Vec<u64>, u64) {
    let mut total_weight = 0;
    let mut weights = Vec::with_capacity(layer.len());

    for image_filename in layer {
        let weight_occurrences: Vec<&str> = RE_WEIGHT
            .captures_iter(image_filename)
            .map(|captures| captures.get(0).unwrap().as_str().trim_start_matches('#'))
            .collect();

        let accumulated_weight: u64 = weight_occurrences
            .iter()
            .map(|&weight_value| weight_value.parse::<u64>().unwrap())
            .sum();

        total_weight += accumulated_weight;
        weights.push(total_weight);
    }

    (weights, total_weight)
}

fn choose_image_with_precomputed_weights<'a>(
    layer: &'a [String],
    weights: &[u64],
    total_weight: u64,
) -> &'a String {
    let mut rng = rand::thread_rng();
    let dist = Uniform::from(0..total_weight);
    let random_value = rng.sample(dist);
    let chosen_index = match weights.binary_search_by(|&probe| probe.cmp(&random_value)) {
        Ok(index) => index,
        Err(index) => index,
    };

    &layer[chosen_index]
}
fn generate_permutations(
    layers: &Vec<Vec<String>>,
    total_supply: usize,
) -> HashMap<u64, Vec<String>> {
    let layer_weights: Vec<_> = layers
        .iter()
        .map(|layer| calculate_weights_and_total(layer))
        .collect();

    let mut rng = rand::thread_rng();
    let mut permutations: HashMap<u64, Vec<String>> = HashMap::new();
    let mut seen_permutations: HashSet<Vec<String>> = HashSet::new();

    while permutations.len() < total_supply {
        let current_permutation: Vec<String> = layers
            .iter()
            .enumerate()
            .zip(&layer_weights)
            .filter_map(|((_index, layer), &(ref weights, total_weight))| {
                if layer.is_empty() {
                    None
                } else if total_weight == 0 {
                    Some(layer.choose(&mut rng).unwrap().to_owned())
                } else {
                    let chosen =
                        choose_image_with_precomputed_weights(layer, weights, total_weight);
                    Some(chosen.to_owned())
                }
            })
            .collect();

        if seen_permutations.insert(current_permutation.clone()) {
            let mut hasher = DefaultHasher::new();
            current_permutation.hash(&mut hasher);
            let hash = hasher.finish();
            permutations.insert(hash, current_permutation);
        }
    }

    permutations
}

fn get_image_paths_recursive(dir: &Path) -> Vec<String> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok) // Ignore errors (like permissions denied)
        .filter(|entry| {
            let entry_path = entry.path();
            if entry_path.is_file() {
                if let Some(extension) = entry_path.extension() {
                    if let Some(ext_str) = extension.to_str() {
                        return ALLOWED_EXTENSION.eq_ignore_ascii_case(ext_str);
                    }
                }
            }
            return false;
        })
        .map(|e| e.path().to_string_lossy().into_owned())
        .collect::<Vec<String>>()
}

fn get_layers_by_traits(traits: Vec<String>) -> Vec<Vec<String>> {
    let mut layers = Vec::<Vec<String>>::new();

    for trait_path in traits.iter() {
        let layers_by_trait = get_image_paths_recursive(Path::new(&trait_path));
        layers.push(layers_by_trait);
    }

    return layers;
}

fn generate_image_and_metadata(
    metadata: HashMap<String, Value>,
    image_paths: Vec<String>,
    output_path: String,
    config_image: Image,
    image_name: usize,
) -> impl FnMut() {
    let images: Vec<(DynamicImage, Attribute)> = image_paths
        .par_iter()
        .map(|path| {
            let img = image::open(&Path::new(&path)).unwrap();
            let filename = Path::new(&path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            let captures = RE_FILENAME.captures(filename);

            let weight_value: f64 = captures
                .as_ref()
                .and_then(|caps| caps.get(2).map(|m| m.as_str().parse().ok()))
                .flatten()
                .unwrap_or(1.0);

            let mut path_parts: Vec<String> = path.split("/").map(|s| s.to_string()).collect();

            path_parts.drain(0..2).for_each(drop);

            for string in path_parts.iter_mut() {
                *string = RE_PATH.replace_all(&string, "").to_string();
            }

            let attribute = Attribute {
                trait_type: path_parts.first().unwrap().to_string(),
                value: path_parts.last().unwrap().to_string(),
                weight: weight_value,
            };

            (img, attribute)
        })
        .collect();
    let width = config_image.width;
    let height = config_image.height;

    let mut combined_image = ImageBuffer::new(width, height);
    // kill me now
    let closure = move || {
        let mut attributes: Vec<Value> = Vec::new();

        for (image, attribute) in &images {
            let mut attribute_map = serde_json::Map::new();
            let attr = attribute.clone();
            attribute_map.insert("trait_type".to_string(), Value::from(attr.trait_type));
            attribute_map.insert("value".to_string(), Value::from(attr.value));
            attributes.push(Value::Object(attribute_map));

            image::imageops::overlay(&mut combined_image, image, 0, 0);
        }

        combined_image
            .save(format!("./{}/{}.png", output_path, image_name))
            .unwrap();

        let mut combined_metadata = metadata.clone();
        combined_metadata.insert("attributes".to_string(), Value::Array(attributes));

        let serialized = to_string_pretty(&combined_metadata).unwrap();

        let mut file = File::create(format!("./{}/{}.json", output_path, image_name)).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
    };

    closure
}

fn get_permutations(layers: &Vec<Vec<String>>, skipped_traits: Option<Vec<String>>) -> usize {
    let regexes = skipped_traits.map(|traits| {
        traits
            .into_iter()
            .map(|pattern| Regex::new(&pattern).unwrap())
            .collect::<Vec<Regex>>()
    });

    layers.iter().fold(1, |acc, layer| {
        let value = if let Some(regexes) = &regexes {
            layer
                .iter()
                .filter(|item| !regexes.iter().any(|regex| regex.is_match(item)))
                .count()
                .max(1)
        } else {
            layer.len().max(1)
        };
        acc * value
    })
}

fn remove_ds_store_files_recursively(folder_path: String) -> std::io::Result<()> {
    for entry in WalkDir::new(folder_path) {
        let entry = entry?;
        if entry.file_name().to_string_lossy() == ".DS_Store" {
            fs::remove_file(entry.path())?;
            println!("Removed file: {}", entry.path().display());
        }
    }
    Ok(())
}

fn remove_pre_existing_output(output_path: String) -> std::io::Result<()> {
    for entry in WalkDir::new(&output_path) {
        let entry = entry.unwrap();
        if entry.path().is_file() {
            fs::remove_file(entry.path()).unwrap();
        }
    }
    Ok(())
}

fn should_include_file(
    forced_combinations: &[ForcedCombo],
    file_path: &str,
    base_path: &str,
) -> bool {
    let path = Path::new(file_path);

    let parent = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|f| f.to_str())
        .unwrap_or(base_path);

    let grandparent = path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .and_then(|f| f.to_str())
        .unwrap_or(base_path);

    let path_parts: Vec<&str> = file_path.split('/').collect();

    let file_name = path_parts.last().unwrap().split('#').next().unwrap();

    let target_layer_to_find = if grandparent.eq(base_path) {
        parent
    } else {
        grandparent
    };

    let forced_combination = forced_combinations.iter().find(|fc| match &fc.layer {
        Layer::Simple(layer) => layer == target_layer_to_find,
        Layer::Complex {
            mainLayer,
            subLayer,
        } => mainLayer == target_layer_to_find || subLayer == target_layer_to_find,
    });

    match forced_combination {
        Some(fc) => match &fc.layer {
            Layer::Simple(layer) => file_name.starts_with(&fc.value) && parent == layer,
            Layer::Complex {
                mainLayer,
                subLayer,
            } => {
                if fc.value == "*" {
                    grandparent == mainLayer && parent.starts_with(subLayer)
                } else {
                    grandparent == mainLayer
                        && parent.starts_with(subLayer)
                        && file_name.starts_with(&fc.value)
                }
            }
        },
        None => true,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let input_path = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "config.json".to_string());
    let file_name = format!("./{}", input_path.as_str());
    let json_file_path = Path::new(&file_name);
    let file = File::open(json_file_path).expect("file not found");
    let config: Config = serde_json::from_reader(file).expect("error while reading");
    let base_path = config.base_path;

    _ = remove_ds_store_files_recursively(base_path.clone());

    let traits = get_entries_by_path_dir(base_path.clone())?;
    let traits_by_config = config
        .layer_folders
        .iter()
        .map(|layer_folder| format!("{}{}", base_path.clone(), layer_folder))
        .collect();

    let ordered_traits = compare_and_verify_traits::<String>(traits, traits_by_config, &base_path)?;

    let all_layers = get_layers_by_traits(ordered_traits);
    let mut permutations: HashMap<u64, Vec<String>> = HashMap::new();
    let mut remaining_layers_for_next_combinations = all_layers.clone();
    let mut possible_permutations: usize = 0;
    let mut layer_count: usize = 0;
    let mut rest_of_items_percentage = config.total_supply;

    if !config.forced_combinations.is_empty() {
        let total_percentage: u32 = config
            .forced_combinations
            .iter()
            .map(|combo| u32::from(combo.percentage))
            .sum();

        if total_percentage > 100 {
            CustomError::TotalPercentageExceeded(
                "The sum of the percentages in the forced combinations exceeds 100%.".to_string(),
            );
        }

        let mut not_included_layers: Vec<Vec<String>> = Vec::new();

        for forced_combination_item in &config.forced_combinations {
            let current_forced_combination_config = &forced_combination_item.combo;
            let current_forced_combination_percentage = forced_combination_item.percentage;
            let mut included_layers: Vec<Vec<String>> = Vec::new();

            for layer_data in &all_layers {
                let mut included = Vec::new();
                let mut not_included = Vec::new();

                for file_path in layer_data {
                    if should_include_file(
                        current_forced_combination_config,
                        file_path,
                        base_path.as_str(),
                    ) {
                        included.push(file_path.clone());
                    } else {
                        not_included.push(file_path.clone());
                    }
                }

                not_included_layers.push(not_included);
                included_layers.push(included);
            }

            for (remaining_layers_for_next_combinations_el, not_included_layers_el) in
                remaining_layers_for_next_combinations
                    .iter_mut()
                    .zip(&not_included_layers)
            {
                if !not_included_layers_el.is_empty() {
                    remaining_layers_for_next_combinations_el.clear();
                    remaining_layers_for_next_combinations_el
                        .extend(not_included_layers_el.iter().cloned());
                }
            }

            let total_items_percentage =
                (config.total_supply * u32::from(current_forced_combination_percentage)) / 100;
            rest_of_items_percentage =
                rest_of_items_percentage.saturating_sub(total_items_percentage);

            permutations.extend(generate_permutations(
                &included_layers,
                total_items_percentage as usize,
            ));
            possible_permutations +=
                get_permutations(&included_layers, config.skipped_traits.clone());

            layer_count = remaining_layers_for_next_combinations.len();
        }

        permutations.extend(generate_permutations(
            &remaining_layers_for_next_combinations,
            rest_of_items_percentage as usize,
        ));
    } else {
        permutations = generate_permutations(&all_layers, config.total_supply as usize);
        possible_permutations = get_permutations(&all_layers, config.skipped_traits.clone());
        layer_count = all_layers.len();
    }

    println!(
        "The number of possible permutations for {} layers is: {}.",
        layer_count, possible_permutations
    );

    if possible_permutations < config.total_supply as usize {
        CustomError::InvalidTotalSupply(config.total_supply.into(), possible_permutations as u64);
    }

    let mut threads = Vec::new();

    _ = fs::create_dir_all(config.output_path.clone());
    _ = remove_pre_existing_output(config.output_path.clone());

    for (index, image_paths) in permutations.into_iter().enumerate() {
        let handle = std::thread::spawn(generate_image_and_metadata(
            config.metadata.clone(),
            image_paths.1,
            config.output_path.clone(),
            config.image,
            index,
        ));
        threads.push(handle);
    }

    for handle in threads {
        let start = Instant::now();
        handle.join().unwrap();
        let duration = start.elapsed();

        println!("Time elapsed in seconds: {:?}", duration);
    }

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImage, Rgba};
    use std::{
        collections::HashSet,
        fs::{self},
    };
    use tempfile::{tempdir, Builder};

    #[test]
    fn test_should_include_file() {
        let base_path = "images";

        let forced_combinations = vec![
            ForcedCombo {
                layer: Layer::Simple("Face".to_string()),
                value: "BasilSynth_V1".to_string(),
            },
            ForcedCombo {
                layer: Layer::Complex {
                    mainLayer: "Hair".to_string(),
                    subLayer: "Black#700".to_string(),
                },
                value: "*".to_string(),
            },
        ];

        let file_path1 = "./images/Face/BasilSynth_V1#25.png";

        assert!(should_include_file(
            &forced_combinations,
            file_path1,
            base_path
        ));

        let file_path3 = "./images/Hair/Black#700/Style2#25.png";

        assert!(should_include_file(
            &forced_combinations,
            file_path3,
            base_path
        ));

        let file_path2 = "./images/Hair/Red#500/Style1#25.png";

        assert!(!should_include_file(
            &forced_combinations,
            file_path2,
            base_path
        ));
    }

    #[test]
    fn test_get_entries_by_path_dir() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();
        let subdir_path1 = dir_path.join("trait1");
        let subdir_path2 = dir_path.join("trait2");

        fs::create_dir(subdir_path1).unwrap();
        fs::create_dir(subdir_path2).unwrap();

        let result = get_entries_by_path_dir(dir.path().to_str().unwrap().to_string());
        assert!(result.is_ok());
        let  traits = result.unwrap();
        assert_eq!(traits.len(), 2);
        assert!(traits[0].contains(&"trait1".to_string()));
        assert!(traits[1].contains(&"trait2".to_string()));

        let file_path1 = dir.path().join("trait1/layer1.png");
        let file_path2 = dir.path().join("trait1/layer2.png");

        File::create(file_path1).unwrap();
        File::create(file_path2).unwrap();

        let result = get_entries_by_path_dir(format!(
            "{}/{}",
            dir.path().to_str().unwrap().to_string(),
            "trait1"
        ));
        assert!(result.is_ok());
        let layers = result.unwrap();
        assert_eq!(traits.len(), 2);
        assert!(layers[0].contains(&"layer1.png".to_string()));
        assert!(layers[1].contains(&"layer2.png".to_string()));

        fs::remove_dir_all(dir.path()).unwrap();
        let result = get_entries_by_path_dir(dir.path().to_str().unwrap().to_string());
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            CustomError::GetEntriesByPath(dir.path().to_str().unwrap().to_string())
        );
    }

    #[test]
    fn test_compare_and_verify_traits() {
        let base_path = "./images";
        let vec1 = vec![
            "back_acc".to_string(),
            "background".to_string(),
            "body".to_string(),
            "face".to_string(),
            "front_acc".to_string(),
        ];
        let vec2 = vec![
            "back_acc".to_string(),
            "background".to_string(),
            "body".to_string(),
            "face".to_string(),
            "front_acc".to_string(),
        ];
        let vec3 = vec![
            "back_acc".to_string(),
            "coverall".to_string(),
            "face".to_string(),
            "front_acc".to_string(),
            "body".to_string(),
        ];
        let vec4 = vec![
            "back_acc".to_string(),
            "body".to_string(),
            "face".to_string(),
            "front_acc".to_string(),
        ];

        assert_eq!(
            compare_and_verify_traits::<String>(vec1.clone(), vec2.clone(), &base_path).unwrap(),
            vec2
        );

        assert!(
            compare_and_verify_traits::<String>(vec1.clone(), vec3.clone(), &base_path).is_err()
        );

        assert!(
            compare_and_verify_traits::<String>(vec1.clone(), vec4.clone(), &base_path).is_err()
        );
    }

    #[test]
    fn test_generate_permutations() {
        let layers = vec![
            (vec!["a".to_string(), "b".to_string(), "c".to_string()]),
            (vec!["1".to_string(), "2".to_string()]),
            (vec!["x".to_string(), "y".to_string(), "z".to_string()]),
        ];
        let total_supply = 18;
        let permutations = generate_permutations(&layers, total_supply);

        assert_eq!(permutations.len(), total_supply);

        for combination in permutations.values() {
            assert_eq!(combination.len(), layers.len());
        }

        let mut hash_set = HashSet::new();
        for combination in permutations.values() {
            let mut hasher = DefaultHasher::new();
            combination.hash(&mut hasher);
            let hash = hasher.finish();
            assert!(!hash_set.contains(&hash));
            hash_set.insert(hash);
        }
    }

    #[test]
    fn test_get_combinations() {
        let layers = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["1".to_string(), "2".to_string(), "3".to_string()],
        ];
        assert_eq!(get_permutations(&layers, None), 6);

        let layers = vec![
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["1".to_string()],
            vec!["x".to_string(), "y".to_string(), "z".to_string()],
        ];
        assert_eq!(get_permutations(&layers, None), 9);

        let layers = vec![
            vec![],
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        ];
        assert_eq!(get_permutations(&layers, None), 3);
        let layers = vec![
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec![],
        ];
        assert_eq!(get_permutations(&layers, None), 3);
    }

    #[test]
    fn test_generate_image_and_metadata() {
        let temp_files = [
            Builder::new().suffix(".png").tempfile().unwrap(),
            Builder::new().suffix(".png").tempfile().unwrap(),
            Builder::new().suffix(".png").tempfile().unwrap(),
        ];

        let mut images = vec![
            DynamicImage::new_rgba8(800, 600),
            DynamicImage::new_rgba8(800, 600),
            DynamicImage::new_rgba8(800, 600),
        ];

        images[0].put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        images[1].put_pixel(0, 0, Rgba([0, 255, 0, 255]));
        images[2].put_pixel(0, 0, Rgba([0, 0, 255, 255]));

        let temp_file_paths: Vec<String> = temp_files
            .iter()
            .enumerate()
            .map(|(i, file)| {
                images[i]
                    .save_with_format(file.path(), image::ImageFormat::Png)
                    .unwrap();
                file.path().to_str().unwrap().to_owned()
            })
            .collect();

        let config_image = Image {
            width: 800,
            height: 600,
        };

        let mut metadata: HashMap<String, Value> = HashMap::new();
        metadata.insert("name".to_string(), Value::from("test dummy data".to_string()));
        metadata.insert(
            "description".to_string(),
            Value::from("test dummy data description".to_string()),
        );

        let image_name = 1;

        let dir = tempdir().expect("Error to create the temp dir");
        let temp_path_str = dir.path().to_str().unwrap().to_owned();

        let mut closure = generate_image_and_metadata(
            metadata.clone(),
            temp_file_paths.clone(),
            temp_path_str.clone(),
            config_image,
            image_name,
        );
        closure();

        let file_path = format!("{}/1.png", temp_path_str.clone());
        assert!(Path::new(&file_path).exists());

        let mut closure2 = generate_image_and_metadata(
            metadata.clone(),
            temp_file_paths.clone(),
            temp_path_str.clone(),
            config_image,
            image_name + 1,
        );
        closure2();

        let file_path2 = format!("{}/2.png", temp_path_str.clone());
        assert!(Path::new(&file_path2).exists());

        let img1 = image::open(&file_path).expect("Failed to open first image");
        let img2 = image::open(&file_path2).expect("Failed to open second image");

        assert_ne!(
            img1.into_bytes(),
            img2.into_bytes(),
            "Images should not be identical"
        );

        dir.close().expect("Error to delete the temp dir");
    }

    #[test]
    fn test_calculate_weights_and_total() {
        let layer = vec![
            "image#100.png".to_string(),
            "image#25.png".to_string(),
            "image#50.png".to_string(),
            "image.png".to_string(),
        ];

        let (weights, total_weight) = calculate_weights_and_total(&layer);

        assert_eq!(weights, vec![100, 125, 175, 175]);
        assert_eq!(total_weight, 175);
    }

    #[test]
    fn test_choose_image_with_precomputed_weights() {
        let layer = vec![
            "image#100.png".to_string(),
            "image#25.png".to_string(),
            "image#50.png".to_string(),
            "image.png".to_string(),
        ];

        let (weights, total_weight) = calculate_weights_and_total(&layer);

        let chosen_image = choose_image_with_precomputed_weights(&layer, &weights, total_weight);

        assert!(layer.contains(chosen_image));
    }

    #[test]
    fn test_get_image_paths_recursive() {
        // Create a temporary directory.
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        // Create subdirectories.
        let subdir1 = dir_path.join("subdir1");
        let subdir2 = dir_path.join("subdir2");
        std::fs::create_dir(&subdir1).unwrap();
        std::fs::create_dir(&subdir2).unwrap();

        // Create files.
        let file1 = dir_path.join("file1.png");
        let file2 = subdir1.join("file2.jpg");
        let file3 = subdir2.join("file3.jpeg");
        let file4 = subdir2.join("file4.txt"); // Non-image file.
        File::create(&file1).unwrap();
        File::create(&file2).unwrap();
        File::create(&file3).unwrap();
        File::create(&file4).unwrap();

        let image_paths = get_image_paths_recursive(dir_path);

        assert!(image_paths.contains(&file1.to_string_lossy().into_owned()));
        assert!(!image_paths.contains(&file2.to_string_lossy().into_owned()));
        assert!(!image_paths.contains(&file3.to_string_lossy().into_owned()));
        assert!(!image_paths.contains(&file4.to_string_lossy().into_owned()));
    }
}
