# RUST NFT GENERATOR

The Rust NFT Generator is a tool designed to generate NFT art from image layers. The layers are loaded into the `images/` folder and then used to generate unique and attractive NFT art.

## Features:

- Automatic generation of NFT art from predefined image layers.
- Ability to define settings via a `config.json` file.
- Creates and saves generated NFTs to the output location specified in the settings.

## Installation

To install and use this NFT Art Generator, please follow these steps:

1. Clone the repository to your local machine.
2. Make sure you have Rust installed. If you don't, you can download and install it following the instructions on the [official Rust page](https://www.rust-lang.org/tools/install).
3. Navigate to the project directory in your terminal.
4. Run the command `cargo build` to build the project.
5. Once built, you can use the command `cargo run` to run the program.

## Usage

To use the NFT Art Generator, please follow these steps:

1. Ensure you have the correct file structure as shown below:

```
YOUR_PROJECT/
├─ images/
│  ├─ trait1_name/
│  │  ├─ file1.png
│  │  ├─ file2.png
│  │  ├─ file3.png
│  │  ├─ ...
│  ├─ trait2_name/
│  │  ├─ file4.png
│  │  ├─ file5.png
│  │  ├─ ...
│  ├─ trait3_name/
│  │  ├─ file6.png
│  │  ├─ ...
│  ├─ ...
```

2. Define your `config.json` file as described below.

## Config File

The `config.json` file is vital for customizing the generation of your NFT art. Here's the file format:

```json
{
  "metadata": {
    "name": "Collection metadata name",
    "description": "Collection metadata description"
  },
  "image": {
    "width": 2000,
    "height": 2000
  },
  "totalSupply": 10,
  "basePath": "./images/",
  "outputPath": "./output/",
  "imageUrl": "rust-nft-art-generator.io",
  "layerFolders": ["Folder1", "Folder2", "Folder3", "Folder4", "Folder5"],
  "forcedCombinations": [
    {
      "combo": [
        {
          "layer": "LayerName",
          "value": "Image"
        },
        {
          "layer": {
            "mainLayer": "LayerName",
            "subLayer": "SubLayerName"
          },
          "value": "*"
        }
      ],
      "percentage": 20
    },
    {
      "combo": [
        {
          "layer": "LayerName",
          "value": "Image"
        },
        {
          "layer": {
            "mainLayer": "LayerName",
            "subLayer": "SubLayerName"
          },
          "value": "*"
        }
      ],
      "percentage": 40
    }
  ]
}
```

Where:

- metadata: Is an object that contains the name and description of your NFT art collection.
- image: Defines the size (width and height) of the generated images.
- totalSupply: The total number of NFTs to be generated.
- basePath: The path of the folder where the layer images are stored. It should end with /.
- outputPath: The path of the folder where the generated images will be saved. It should end with /.
- imageUrl: The base URL where the generated images will be hosted.
- layerFolders: A list of folder names that contain the image layers to be used for NFT art generation. The order of the folders here is important, as it determines the order in which the layers will be applied to generate the final art.
- forcedCombinations: A structured list of layers that will be combined with each other in all permutations.

## Uniform Distribution

The uniform distribution is a probability distribution that allows for randomly selecting values within a range in a fair and equal manner. It ensures that all possible values have an equal chance of being selected.

Think of a common six-sided dice. When you roll the dice, each face has the same probability of appearing. The uniform distribution ensures that all possible outcomes are equally likely. This means that each number from 1 to 6 has an equal chance of being the result of the roll.

Similarly, the uniform distribution can be applied to any range of values, not just the numbers on a dice. If we have a range of values from 1 to 100 and we want to randomly select a number, the uniform distribution will guarantee that each number from 1 to 100 has an equal probability of being selected.

In essence, the uniform distribution is like having a "box" of elements where each element has an equal probability of being chosen. This allows for fair and equitable selection within the specified range of values.

Now to our application, imagine we have three images with weights [2, 3, 5]. The total_weight would be 10. We could associate the first string with the range [0, 2), the second with [2, 5), and the third with [5, 10). Then we generate a random number in the range [0, 10). Depending on where that number falls, we choose the associated string.

The effect of this is that strings associated with larger weights are more likely to be chosen than those with smaller weights, and the likelihood of a string being chosen is proportional to its weight relative to the total_weight. Thus, it provides a method of randomly sampling from a set where each element has a different probability of being chosen.

## Forced Combinations

This mechanism allows you to define combinations between layers, where the selection of each element will be prioritized over the rest. Additionally, you can specify a percentage that indicates the proportion of the total output that should contain each forced combination.

Here's an example of how to configure it:

```json
"forcedCombinations": [
    {
      "combo": [
        {
          "layer": "Face",
          "value": "Furian_ErikScar_V1_Final"
        },
        {
          "layer": {
            "mainLayer": "Hair",
            "subLayer": "White"
          },
          "value": "*"
        }
      ],
      "percentage": 20
    },
    {
      "combo": [
        {
          "layer": "Face",
          "value": "BilboScar_v1"
        },
        {
          "layer": {
            "mainLayer": "Hair",
            "subLayer": "Misc"
          },
          "value": "*"
        }
      ],
      "percentage": 40
    }
]
```

In this configuration, 20% of the total output will combine the `Face/Furian_ErikScar_V1_Final.png` image and any image from the `White` folder under the `Hair` folder with other layers, and 40% will combine the `Face/BilboScar_v1.png` image and any image from the `Misc` folder under the `Hair` folder with other layers.

The `layer` key within each item in the `combo` array defines the layer(s) to include, and the `value` key specifies the specific file(s) or any file (`*`). The `percentage` key defines the proportion of the total output that should contain this combination.

When the `layer` is a simple string (e.g., `"Face"`), it refers to a directory in the `images` folder, and the `value` should be the name of an image file (without the extension) in that directory.

When the `layer` is a complex object with `mainLayer` and `subLayer`, it refers to a directory structure within the `images` folder. The `mainLayer` is the parent directory, and the `subLayer` is a subdirectory within the `mainLayer`. In this case, the `value` can be the name of a specific image file (without the extension) in the `subLayer` directory, or it can be `*` to represent any image within the `subLayer` directory.

The sum of all percentages in the forced combinations should not exceed 100%. If it does, an error will be thrown.

With forced combinations and their percentages, you can ensure certain combinations are always included in a specific proportion, while still maintaining variety in the other layers.

## Contributing

We highly appreciate contributions. If you'd like to contribute, please follow these steps:

1. Fork the repository.
2. Create your feature branch (git checkout -b feature/YourFeature).
3. Commit your changes (git commit -m 'Add some feature').
4. Push to the branch (git push origin feature/YourFeature).
5. Open a pull request.

Please make sure to update tests as appropriate.
