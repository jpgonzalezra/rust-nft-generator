use image::{DynamicImage, ImageBuffer};
use rand::seq::SliceRandom;
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{read_dir, File};
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::Instant;
use std::{fmt, fs};
use walkdir::WalkDir;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Metadata {
    name: String,
    description: String,
}

#[derive(Debug, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "camelCase")]
struct Image {
    width: u32,
    height: u32,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Config {
    metadata: Metadata,
    image: Image,
    total_supply: u16,
    base_path: String,
    output_path: String,
    image_url: String,
    layer_folders: Vec<String>,
}

#[derive(Debug)]
pub enum CustomError {
    GetEntriesByPath(String),
    InvalidTrait(String),
    InvalidTotalSupply(u64, u64),
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
    mut traits_by_path: Vec<T>,
    mut traits_by_config: Vec<T>,
) -> Result<Vec<T>, CustomError> {
    if traits_by_path.len() != traits_by_config.len() {
        return Err(CustomError::InvalidTrait(format!(
            "[traits_by_path: {} traits_by_config: {}]",
            traits_by_path.len(),
            traits_by_config.len()
        )));
    }

    let out = traits_by_config.clone();

    traits_by_path.sort();
    traits_by_config.sort();

    if traits_by_path != traits_by_config {
        return Err(CustomError::InvalidTrait(
            "[traits_by_path and traits_by_config are different]".to_string(),
        ));
    }

    Ok(out)
}

fn generate_combinations(layers: &[Vec<String>], total_supply: usize) -> HashMap<u64, Vec<String>> {
    let mut rng = rand::thread_rng();
    let mut combinations: HashMap<u64, Vec<String>> = HashMap::new();

    while combinations.len() < total_supply {
        let current_combination = layers
            .iter()
            .map(|l| l.choose(&mut rng).unwrap().clone())
            .collect::<Vec<String>>();

        let mut hasher = DefaultHasher::new();
        current_combination.hash(&mut hasher);
        let hash = hasher.finish();

        if !combinations.contains_key(&hash) {
            combinations.insert(hash, current_combination);
        }
    }

    combinations
}

fn get_layers_by_traits(traits: Vec<String>) -> Vec<Vec<String>> {
    let mut layers = Vec::<Vec<String>>::new();

    for trait_path in traits.iter() {
        let layers_by_trait = get_entries_by_path_dir(trait_path.clone()).unwrap();
        layers.push(layers_by_trait);
    }

    return layers;
}

fn generate_image(
    image_paths: Vec<String>,
    output_path: String,
    config_image: Image,
    image_name: usize,
) -> impl FnMut() {
    let images: Vec<DynamicImage> = image_paths
        .par_iter()
        .map(|path| image::open(&Path::new(path)).unwrap())
        .collect();

    let width = config_image.width;
    let height = config_image.height;

    let mut combined_image = ImageBuffer::new(width, height);

    let closure = move || {
        for image in &images {
            image::imageops::overlay(&mut combined_image, image, 0, 0);
        }

        combined_image
            .save(format!("./{}/{}.png", output_path, image_name))
            .unwrap();
    };

    closure
}

fn get_combinations(layers: &Vec<Vec<String>>) -> usize {
    layers.iter().fold(1, |acc, layer| {
        let value = layer.len().max(1);
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

fn main() -> Result<(), Box<dyn Error>> {
    // Validate arguments

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
    let ordered_traits = compare_and_verify_traits::<String>(traits, traits_by_config)?;

    let layers = get_layers_by_traits(ordered_traits);

    let possible_combinations = get_combinations(&layers);
    println!(
        "The number of possible combinations for {} layers is: {}.",
        layers.len(),
        get_combinations(&layers)
    );
    if possible_combinations < config.total_supply.into() {
        println!("Use exception")
    }

    let combinations = generate_combinations(&layers, config.total_supply.into());

    let mut threads = Vec::new();

    _ = fs::create_dir_all(config.output_path.clone());
    for (index, image_paths) in combinations.into_iter().enumerate() {
        let handle = std::thread::spawn(generate_image(
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
    fn test_get_entries_by_path_dir() {
        let dir = tempdir().unwrap();
        let subdir_path1 = dir.path().join("trait1");
        let subdir_path2 = dir.path().join("trait2");

        fs::create_dir(subdir_path1).unwrap();
        fs::create_dir(subdir_path2).unwrap();

        let result = get_entries_by_path_dir(dir.path().to_str().unwrap().to_string());
        assert!(result.is_ok());
        let traits = result.unwrap();
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
        let vec1 = vec!["back_acc", "face", "front_acc", "body", "background"];
        let vec2 = vec!["back_acc", "background", "body", "face", "front_acc"];
        let vec3 = vec!["back_acc", "coverall", "face", "front_acc", "body"];
        let vec4 = vec!["back_acc", "body", "face", "front_acc"];

        // success
        assert_eq!(
            compare_and_verify_traits(vec1.clone(), vec2.clone()).unwrap(),
            vec2
        );

        // different elements
        assert!(compare_and_verify_traits(vec1.clone(), vec3.clone()).is_err());

        // different sizes
        assert!(compare_and_verify_traits(vec1.clone(), vec4.clone()).is_err());
    }

    #[test]
    fn test_generate_combinations() {
        let layers = vec![
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["x".to_string(), "y".to_string(), "z".to_string()],
        ];
        let total_supply = 18;
        let combinations = generate_combinations(&layers, total_supply);

        assert_eq!(combinations.len(), total_supply);

        for combination in combinations.values() {
            assert_eq!(combination.len(), layers.len());
        }

        let mut hash_set = HashSet::new();
        for combination in combinations.values() {
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
        assert_eq!(get_combinations(&layers), 6);

        let layers = vec![
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["1".to_string()],
            vec!["x".to_string(), "y".to_string(), "z".to_string()],
        ];
        assert_eq!(get_combinations(&layers), 9);

        let layers = vec![
            vec![],
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        ];
        assert_eq!(get_combinations(&layers), 3);
        let layers = vec![
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec![],
        ];
        assert_eq!(get_combinations(&layers), 3);
    }

    #[test]
    fn test_generate_image() {
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
        let image_name = 1;

        let dir = tempdir().expect("Error to create the temp dir");
        let temp_path_str = dir.path().parent().unwrap().to_str().unwrap().to_owned();
        let mut clousure = generate_image(
            temp_file_paths.clone(),
            temp_path_str.clone(),
            config_image,
            image_name,
        );
        clousure();

        let file_path = format!("./{}/1.png", temp_path_str.clone());
        assert!(Path::new(&file_path).exists());

        fs::remove_dir_all(dir).expect("Error to delete the temp dir");
    }
}
