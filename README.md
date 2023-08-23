<!--
<p align="center">
  <a href="">
    <img height="150" src="./icon/icon.svg">
  </a>
</p>
-->
<h1 align="center">NWLKN Toolset</h1>
<h3 align="center">nlwkn-rs</h3>
<p align="center">
  <b>📑 Tools for handling water rights data from Niedersachsen's Cadenza database.</b>
</p>

<br>

<p align="center">
  <a>
    <img alt="Version" src="https://img.shields.io/badge/version-1.0.0-blue?style=for-the-badge"/>
  </a>
</p>


## About
`nlwkn-rs` is a collection of tools aimed at handling water rights data from the 
"niedersächsischen Landesdatenbank für wasserwirtschaftliche Daten" available at 
[Cadenza](http://www.wasserdaten.niedersachsen.de/cadenza/). 
The platform provides an extensive list of active water rights in 
Lower Saxony, Germany, which can be viewed in a tabular form or visualized on 
a map.

## Project Structure
`lib`: Contains shared code that all tools utilize.
Each tool resides in its own dedicated directory:

- `fetcher`: 
  Contains the tool to fetch water rights in PDF format from the Cadenza 
  database.

- `parser`: 
  Houses the tool to parse these PDF reports and enrich them using an XLSX table 
  that can be downloaded from the Cadenza portal.

For a more detailed overview and instructions specific to each tool, please 
refer to the README in their respective directories.

## Installation and Usage
### Prerequisites:

Make sure you have Rust and Cargo installed on your machine. 
If not, you can get them from [rust-lang.org](https://rust-lang.org).

### Clone the repository:

```shell
git clone https://github.com/[your-username]/nlwkn-rs.git
cd nlwkn-rs
```

### Building the project:

```shell
cargo build --release
```

Refer to individual tool directories for usage instructions.

## Disclaimer
This toolset is not officially affiliated with or endorsed by the 
"niedersächsischen Landesdatenbank für wasserwirtschaftliche Daten" or any 
related organizations.

