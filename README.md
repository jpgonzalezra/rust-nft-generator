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
  "layerFolders": [
    "Folder1",
    "Folder2",
    "Folder3",
    "Folder4",
    "Folder5"
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


## Contributing

We highly appreciate contributions. If you'd like to contribute, please follow these steps:

1- Fork the repository.
2- Create your feature branch (git checkout -b feature/YourFeature).
3- Commit your changes (git commit -m 'Add some feature').
4- Push to the branch (git push origin feature/YourFeature).
5- Open a pull request.

Please make sure to update tests as appropriate.


