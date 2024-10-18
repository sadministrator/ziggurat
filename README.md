# PDF and EPUB Translator
A command-line utility written in Rust that translates PDF and EPUB files into different languages using the Google Translate API.

## Features
- Supports translation of PDF and EPUB files
- Uses Google Translate API for accurate translations
- Simple command-line interface
- Verbose mode for detailed operation logs

## Prerequisites
- Google Cloud account with Translation API enabled
- Google API key

## Installation
1. Clone this repository:
```
git clone https://github.com/sadministrator/ziggurat
cd ziggurat
```

2. Install the project:
`cargo install --release --path .`

## Configuration
You can provide a Google API key in one of several ways (listed in order of priority):
1. Pass it in the CLI command with `--api-key <YOUR_API_KEY>`.
2. Pass in the path to a JSON config file which contains the key with `--config /path/to/your/key`. Config file contents should look like `{"api_key": "YOUR_API_KEY" }`.
3. Create a `.env` file in the project root directory and add `ZIGGURAT_API_KEY=<YOUR_API_KEY>`.
4. Set it as an environment variable with `export ZIGGURAT_API_KEY`.

## Usage
`ziggurat [OPTIONS] --input <INPUT> --output <OUTPUT> --to <TO>`

### Required Flags
- `-i, --input <INPUT>`: Input file (PDF or EPUB)
- `-o, --output <OUTPUT>`: Output file
- `--to <TO>`: Target language code (e.g., 'es' for Spanish, 'fr' for French)

### Options:
- `--api-key <API_KEY>`: API key
- `-v, --verbose`: Enable verbose mode
- `-h, --help`: Print help
- `-V, --version`: Print version

### Example:
`ziggurat --input book.pdf --output libro.pdf --to es`

This command translates `book.pdf` to Spanish and saves the result as `libro.pdf`.

## Supported Languages
This utility supports all languages available in the Google Translate API. Use the appropriate language code when specifying the target language.

## Notes
- The translation quality depends on the Google Translate API.
- Processing large files may take some time and consume API quota.