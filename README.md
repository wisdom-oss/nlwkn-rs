<!--
<p align="center">
  <a href="">
    <img height="150" src="./icon/icon.svg">
  </a>
</p>
-->
<h1 align="center">NLWKN Toolset</h1>
<h3 align="center">nlwkn-rs</h3>
<p align="center">
  <b>📑 Tools for handling water rights data from Lower Saxony's Cadenza database.</b>
</p>

<br>

<p align="center">
  <a>
    <img alt="Version" src="https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fwisdom-oss%2Fnlwkn-rs%2Fmain%2FCargo.toml&query=package.version&style=for-the-badge&label=version&color=blue"/>
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

- [`fetcher`](./fetcher/README.md): 
  Contains the tool to fetch water rights in PDF format from the Cadenza 
  database.

- [`parser`](./parser/README.md): 
  Houses the tool to parse these PDF reports and enrich them using an XLSX table 
  that can be downloaded from the Cadenza portal.

- [`adapter`](./adapter/README.md):
  A tool to adapt the data types that `nlwkn-rs` is working on and reformat it 
  for other tools or people to use.

- [`exporter`](./exporter/README.md):
  Exporter for the fully parsed water rights into a 
  [PostgreSQL](https://www.postgresql.org) database.

For a more detailed overview and instructions specific to each tool, please 
refer to the README in their respective directories.

## Installation and Usage

### Download from Releases

Precompiled binaries can be found on the 
[releases page](https://github.com/wisdom-oss/nlwkn-rs/releases).
They are available for Linux and Windows 64 bit.

Refer to individual tool READMEs for usage instructions:
[fetcher](./fetcher/README.md),
[parser](./parser/README.md),
[adapter](./adapter/README.md),
[exporter](./exporter/README.md).

### Compile it yourself

#### Prerequisites:

Make sure you have Rust and Cargo installed on your machine. 
If not, you can get them from [rust-lang.org](https://rust-lang.org).

<!-- TODO: add section about using as lib -->

#### Clone the repository:

```shell
git clone https://github.com/[your-username]/nlwkn-rs.git
cd nlwkn-rs
```

#### Building the project:

```shell
cargo build --release
```

Refer to individual tool directories for usage instructions.

## Using nlwkn-rs as a library
Although `nlwkn-rs` is not available on crates.io, you can still use its types 
or general common codebase as a library by adding it to your cargo 
dependencies via the git key. 

Add the following to your Cargo.toml file:
```toml
[dependencies]
nlwkn-rs = { git = "https://github.com/wisdom-oss/nlwkn-rs.git" }
```

## Disclaimer
This toolset is not officially affiliated with or endorsed by the 
"niedersächsischen Landesdatenbank für wasserwirtschaftliche Daten" or any 
related organizations.

