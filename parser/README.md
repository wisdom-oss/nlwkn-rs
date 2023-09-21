<h1 align="center">NLWKN Parser</h1>
<h3 align="center">nlwkn-rs</h3>
<p align="center">
  <b>🔍 Transforming water right PDF reports into structured JSON data.</b>
</p>
<br>

## Introduction
The `parser` tool stands as a cornerstone within the `nlwkn-rs` toolset. 
Its primary objective is to metamorphose the human-centric PDF water right 
reports into a machine-friendly JSON format. 
This transformation is pivotal for subsequent data analysis and processing.

## The Case for JSON
JSON was our format of choice due to its inherent versatility. 
It supports nested data structures, is both human and machine-readable, and is 
ubiquitously recognized across various programming languages. 
While we currently don't possess a formal JSON schema, the Rust types in the 
`lib` directory serve as our ground-truth.

## The Parsing Odyssey
### From Raw PDF to Text Block
The initial state of the report is a raw PDF. 
Parsing a PDF is intricate due to its nature. 
The underlying format of a PDF is essentially a list of rendering instructions. 
By focusing on specific operations, we extract meaningful data into the 
`TextBlock` structure. 
This structure captures essential details like coordinates, font properties, 
fill color, and content.

The `TextBlock` structure is defined as:

```rust
#[derive(Debug, Default)]
pub struct TextBlock {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub fill_color: Option<(f32, f32, f32)>,
    pub content: Option<String>
}
```

The transition from raw PDF to the text block involves interpreting the PDF's 
underlying list of instructions. 
By focusing on specific operations, such as `BT`, `Tm`, `Tf`, `rg`, `Tj`, `ET`, 
and `BT`, we can extract meaningful data.

### Key-Value Representation
Given the design of the PDFs, most data is presented in a key-value style table. 
By leveraging the distinct visual properties (e.g., bold keys), we can extract 
this data into a `KeyValueRepr` structure.

### Grouped Key-Value Representation
To further simplify data extraction, we pre-group the key-value pairs. 
This grouping divides the data into:

- **Root**: 
  Pertains to the water right itself.

- **Departments**: 
  Contains details about various departments and usage locations.

- **Annotation**: 
  Unstructured text annotations found at the end of the water right.

### Final Parsing
Each element of the grouped key-value representation is parsed using dedicated 
functions. 
These functions utilize extensive Rust match statements to ensure data is 
correctly categorized. 
Any unknown keys trigger an error, ensuring no data detail is overlooked.

After parsing, the initial XLSX table enriches the extracted data, providing a 
comprehensive dataset.

## Usage
To utilize the parser, follow the command structure below:

```
NLWKN Water Right Parser

Usage: parser.exe [OPTIONS] <XLSX_PATH> [DATA_PATH]

Arguments:
<XLSX_PATH>  Path to cadenza-provided xlsx file
[DATA_PATH]  Path to data directory [default: data]

Options:
--no <WATER_RIGHT_NO>  Parse specific water right number report
-h, --help                 Print help
-V, --version              Print version
```

## Output
Upon completion, the parser provides a detailed TOML-formatted report. 
This report offers insights into the parsing process, highlighting any issues 
encountered and the overall success rate. 
The structured format of this report facilitates integration with CI/CD 
pipelines.

```toml
# Broken PDF files which cannot be loaded.
# Could be due to corrupted or incompatible files.
[broken]
count = 0
output_file = 'data\broken-reports.json'

# Reports with parsing issues.
# First issue with it's respective water right number.
[parsing_issues]
count = 0
output_file = 'data\parsing-issues.json'

# Reports where data could only be extracted from the PDF file.
# XLSX data might be missing.
[pdf_only]
count = 0
output_file = 'data\pdf-only-reports.json'

# Reports parsed and enriched with both PDF and XLSX data.
[reports]
count = 53035
output_file = 'data\reports.json'
```
