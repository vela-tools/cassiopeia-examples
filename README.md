<p align="center">
  <img src="docs/assets/vela.png" alt="" width="88" height="88">
</p>

<h1 align="center">Cassiopeia Examples</h1>

<p align="center">
  <strong>One real public dataset per example, mapped end to end into <a href="https://ngsi-ld.org/">NGSI-LD</a> entities.</strong>
</p>

<p align="center">
  <a href="#run-an-example">Run an example</a> ·
  <a href="#the-examples">The examples</a> ·
  <a href="#add-an-example">Add an example</a> ·
  <a href="https://vela-tools.github.io/cassiopeia/">Documentation</a> ·
  <a href="#community">Community</a> ·
  <a href="#license">License</a>
</p>

<p align="center">
  <a href="https://github.com/vela-tools/cassiopeia-examples/actions/workflows/examples.yaml"><img alt="examples" src="https://github.com/vela-tools/cassiopeia-examples/actions/workflows/examples.yaml/badge.svg"></a>
  <a href="https://github.com/vela-tools/cassiopeia-examples/actions/workflows/test.yaml"><img alt="tests" src="https://github.com/vela-tools/cassiopeia-examples/actions/workflows/test.yaml/badge.svg"></a>
  <a href="NOTICE.md"><img alt="licenses: MIT-0, CC-BY-4.0, EUPL-1.2" src="https://img.shields.io/badge/licenses-MIT--0%20%C2%B7%20CC--BY--4.0%20%C2%B7%20EUPL--1.2-blue"></a>
  <img alt="ngsi-ld v1.9.1" src="https://img.shields.io/badge/NGSI--LD-v1.9.1-orange">
</p>

<p align="center">
  <sub>Part of <a href="https://github.com/vela-tools">Vela Tools</a> · Based on EU open standards 🇪🇺 · Built in Ljubljana, Slovenia 🇸🇮 by SenLab d.o.o.</sub>
</p>

---

## What this is

Worked examples for [Cassiopeia](https://github.com/vela-tools/cassiopeia), the pipeline that turns CSV, JSON, GeoJSON, KML/KMZ, XML, ESRI Shapefiles, and GRIB into validated NGSI-LD entities.

There are 31 of them. One example is one complete run against a real public dataset: the mapping files, the commands, the output to expect, and the prose that explains why the mapping is shaped the way it is. Nothing here is a fragment. Any machine with Cassiopeia on it, or with Docker or Podman, runs any of them from start to finish.

The datasets are real because the awkward parts of a mapping come from real data.

## Run an example

Every page carries its commands in full. Most examples need `curl` and either Cassiopeia or a container runtime, and a few also want `jq`, `awk`, `zip`, or `docker compose`. The page says which.

```bash
cd examples/01-json-field-mapping
mkdir -p data
curl --fail --location --output data/element.json https://raw.githubusercontent.com/andrejewski/periodic-table/master/data.json
cassiopeia map --input data/element.json --mapping element.json5 --type json --output out --context none
```

That writes `out/ChemicalElement.json`, an array of 118 entities in NGSI-LD's normalized representation. One of them:

```json
{
    "id": "urn:ngsi-ld:ChemicalElement:H",
    "type": "ChemicalElement",
    "name": {
        "type": "Property",
        "value": "Hydrogen"
    },
    "atomicNumber": {
        "type": "Property",
        "value": 1
    },
    "group": {
        "type": "Property",
        "value": "nonmetal"
    },
    "standardState": {
        "type": "Property",
        "value": "gas"
    }
}
```

The runner does the same three steps from the repository root, then checks the output against what the page claims:

```bash
cargo run -- run 01
cargo run -- run 01 --runtime docker
cargo run -- run 01 --runtime podman
```

In container mode it forwards to `ghcr.io/vela-tools/cassiopeia:latest` with the example directory mounted at `/data` and set as the working directory. Relative paths then mean the same thing either way, and an example never needs a second set of commands. [Running an example](docs/running-an-example.md) covers the datasets, the schema catalog, Podman's differences from Docker, and the examples that need a broker.

## The examples

The order is a reading order: each example builds on the ones before it. Start at 01 and read forward, or take whichever one matches the data in front of you.

<!-- examples:start -->

<img src="https://img.shields.io/badge/format-json-d97706" alt="format: json" height="20" align="right"> **[JSON field mapping](examples/01-json-field-mapping/example.md)** → `ChemicalElement`

Map a JSON array to ChemicalElement entities and type one copied field as a number.

<img src="https://img.shields.io/badge/format-json-d97706" alt="format: json" height="20" align="right"> **[ID collisions](examples/02-json-id-collision/example.md)** → `City`

See how a weak city ID merges records, then add a geohash from the coordinates.

<img src="https://img.shields.io/badge/format-geojson-2563eb" alt="format: geojson" height="20" align="right"> **[GeoJSON Smart Data Model](examples/03-geojson-smart-data-model/example.md)** → `OffStreetParking`

Map GeoJSON to OffStreetParking and validate it against the published schema.

<img src="https://img.shields.io/badge/format-csv-1d6f42" alt="format: csv" height="20" align="right"> **[CSV conditionals](examples/04-csv-conditionals/example.md)** → `BikeHireDockingStation`

Read a semicolon-delimited CSV, translate codes, and build an address object.

<img src="https://img.shields.io/badge/format-geojson-2563eb" alt="format: geojson" height="20" align="right"> **[Attribute guards](examples/05-geojson-attribute-guards/example.md)** → `OffStreetParking`

Combine conditional values with constants and omit invalid source values.

<img src="https://img.shields.io/badge/format-csv-1d6f42" alt="format: csv" height="20" align="right"> **[Messy CSV headers](examples/06-csv-messy-headers/example.md)** → `MonthlyPrecipitationObserved`

Address unusual Latin-1 CSV headers and parse comma decimals.

<img src="https://img.shields.io/badge/format-csv-1d6f42" alt="format: csv" height="20" align="right"> **[observedAt observations](examples/07-csv-observed-at/example.md)** → `GoldPriceObserved`

Keep monthly measurements under one entity ID with observedAt.

<img src="https://img.shields.io/badge/format-json-d97706" alt="format: json" height="20" align="right"> **[LanguageProperty](examples/08-json-language-property/example.md)** → `Region`

Build a LanguageProperty and read a hyphenated source key.

<img src="https://img.shields.io/badge/format-csv-1d6f42" alt="format: csv" height="20" align="right"> **[Manifests](examples/09-csv-manifest/example.md)** → `Airport` `Airline`

Use a manifest to map two headerless CSV files in one run.

<img src="https://img.shields.io/badge/format-json-d97706" alt="format: json" height="20" align="right"> **[JSON relationships](examples/10-json-relationships/example.md)** → `Region` `Subregion` `Country` `State`

Connect region, subregion, country, and state entities.

<img src="https://img.shields.io/badge/format-csv-1d6f42" alt="format: csv" height="20" align="right"> **[CSV relationship graph](examples/11-csv-relationship-graph/example.md)** → `Flight` `Airport` `Airline` `AircraftModel` `Country`

Link routes, airports, airlines, aircraft models, and countries.

<img src="https://img.shields.io/badge/format-csv-1d6f42" alt="format: csv" height="20" align="right"> **[ListRelationship](examples/12-csv-list-relationship/example.md)** → `Flight` `Airport` `Airline` `AircraftModel` `Country`

Use a ListRelationship when one route has several aircraft models.

<img src="https://img.shields.io/badge/format-json-d97706" alt="format: json" height="20" align="right"> **[Synthetic entities](examples/13-json-synthetic-entities/example.md)** → `Mountain` `Country`

Create country entities from a mountain's country field.

<img src="https://img.shields.io/badge/format-xml-db2777" alt="format: xml" height="20" align="right"> **[XML unit codes](examples/14-xml-unit-code/example.md)** → `WeatherObserved`

Map XML measurements with unitCode and observedAt.

<img src="https://img.shields.io/badge/format-shapefile-92400e" alt="format: shapefile" height="20" align="right"> **[Shapefile enums](examples/15-shapefile-enum-decoding/example.md)** → `Port`

Read a zipped shapefile and decode its coded columns.

<img src="https://img.shields.io/badge/format-kmz-9333ea" alt="format: kmz" height="20" align="right"> **[KMZ namespacing](examples/16-kmz-folder-namespacing/example.md)** → `ParkPointOfInterest`

Package a KML as a KMZ, namespace its records by folder, and pass placemark geometry through.

<img src="https://img.shields.io/badge/format-kml-7c3aed" alt="format: kml" height="20" align="right"> **[KML collections](examples/17-kml-folder-collections/example.md)** → `CitiBikeStation` `BikeRentalShop` `Sightseeing` `PublicRestroom`

Route four KML folders to four mappings.

<img src="https://img.shields.io/badge/format-grib1-0f766e" alt="format: grib1" height="20" align="right"> **[GRIB1 derived values](examples/18-grib1-derived-values/example.md)** → `WeatherObserved`

Derive weather values from GRIB1 components.

<img src="https://img.shields.io/badge/format-grib2-0d9488" alt="format: grib2" height="20" align="right"> **[GRIB2 byte ranges](examples/19-grib2-byte-range/example.md)** → `WeatherObserved`

Fetch selected GRIB2 fields by byte range.

<img src="https://img.shields.io/badge/format-csv-1d6f42" alt="format: csv" height="20" align="right"> **[Local @context](examples/20-csv-at-context/example.md)** → `ExoPlanet` `Star`

Define a JSON-LD context for an invented model, and materialise its host star as a second entity.

<img src="https://img.shields.io/badge/format-json-d97706" alt="format: json" height="20" align="right"> **[JSON datasetId](examples/21-json-dataset-id/example.md)** → `WeatherForecast`

Keep forecasts from several models under one attribute.

<img src="https://img.shields.io/badge/format-csv-1d6f42" alt="format: csv" height="20" align="right"> **[Temporal to broker](examples/22-csv-broker-temporal/example.md)** → `TropicalCyclone`

Fold a track and deliver it to Scorpio.

<img src="https://img.shields.io/badge/format-json-d97706" alt="format: json" height="20" align="right"> **[Scheduling](examples/23-json-scheduling/example.md)** → `AirQualityObserved`

Poll a sensor feed and upsert entities on a schedule.

<img src="https://img.shields.io/badge/format-json-d97706" alt="format: json" height="20" align="right"> **[ListProperty](examples/24-json-list-property/example.md)** → `BicycleCounter`

Store hourly counts in an ordered ListProperty.

<img src="https://img.shields.io/badge/format-geojson-2563eb" alt="format: geojson" height="20" align="right"> **[JsonProperty](examples/25-geojson-json-property/example.md)** → `WeatherAlert`

Keep an alert's changing parameters object in a JsonProperty.

<img src="https://img.shields.io/badge/format-csv-1d6f42" alt="format: csv" height="20" align="right"> **[VocabProperty](examples/26-csv-vocab-property/example.md)** → `UrbanMobilityPoint`

Turn an OpenStreetMap tag into a VocabProperty IRI.

<img src="https://img.shields.io/badge/format-csv-1d6f42" alt="format: csv" height="20" align="right"> **[Custom schemas](examples/27-csv-custom-schema/example.md)** → `ExoPlanet` `Star`

Validate two invented models against two local JSON Schemas in one run.

<img src="https://img.shields.io/badge/format-json-d97706" alt="format: json" height="20" align="right"> **[Advanced schema validation](examples/28-json-advanced-schema/example.md)** → `FoodProduct`

Validate wrappers and metadata in normalized output.

<img src="https://img.shields.io/badge/format-csv-1d6f42" alt="format: csv" height="20" align="right"> **[Multi-attribute relationships](examples/29-csv-multi-attribute-relationship/example.md)** → `Flight` `Airport` `Airline` `AircraftModel` `Country`

Give a Flight one servesAirport name for its departure and arrival airports, distinguished by datasetId.

<img src="https://img.shields.io/badge/format-csv-1d6f42" alt="format: csv" height="20" align="right"> **[Nested relationships](examples/30-csv-nested-relationship/example.md)** → `Movie`

Give a Movie's hasLeadActor relationship a nested playsCharacter relationship and a billingOrder property.

<img src="https://img.shields.io/badge/format-geojson-2563eb" alt="format: geojson" height="20" align="right"> **[Geometry conversion](examples/31-geojson-geometry-conversion/example.md)** → `Region`

Demote an administrative region's MultiPolygon to the largest Polygon and derive a map-pin centroid beside it.

<!-- examples:end -->

## The runner

`tools/runner` is a small Rust binary, and the only thing that reads `example.toml`. Authors and continuous integration use it. A reader never has to:

```bash
cargo run -- list                              # every example, with its summary
cargo run -- list --matrix                     # every example, as JSON, for the workflow
cargo run -- fetch 01                          # download the datasets
cargo run -- run 01 --skip-verify              # fetch and run, leaving the output unchecked
cargo run -- run 01 --drift-report drift.json  # record what a live source has drifted to
cargo run -- verify 01                         # check an existing out/ against the descriptor
cargo run -- check                             # every page still documents the run it declares
cargo run -- index --write                     # regenerate the index above
cargo run -- metadata                          # sidebar metadata for the documentation site
```

A run ends one of three ways. It passes; it fails, and the example is broken; or it passes with drift, meaning a dataset the descriptor calls `live` has moved under a mapping that still works. Drift is reported and leaves the exit code alone.

`check` is what keeps a page and its descriptor in step. The command lives once, in `example.toml`, and the page's code blocks are derived from it. A page that no longer matches fails the check.

## Add an example

[Authoring an example](docs/authoring-an-example.md) defines the layout, the descriptor keys, and what the prose has to cover. New examples follow it from the first commit.

## Contributing

Cassiopeia is not accepting external contributions right now, this repository included. Reports are another matter, and all of them are welcome: a dataset that moved, a mapping that no longer matches its source, an output the page describes wrongly, a technique with no example. [CONTRIBUTING.md](CONTRIBUTING.md) says why the door is shut and what to put in the report.

Never report a security vulnerability through a public issue. Follow the [Cassiopeia security policy](https://github.com/vela-tools/cassiopeia/security/policy) instead.

## Community

- Issues: [GitHub Issues](../../issues)
- Guides: [vela-tools.github.io/cassiopeia](https://vela-tools.github.io/cassiopeia/)
- Cassiopeia itself: [vela-tools/cassiopeia](https://github.com/vela-tools/cassiopeia)
- Newsletter: [velacontext.com/newsletter](https://velacontext.com/newsletter)
- General contact: info@velacontext.com
- Security: security@velacontext.com, see the [security policy](https://github.com/vela-tools/cassiopeia/security/policy)
- Code of conduct: [Contributor Covenant](https://github.com/vela-tools/cassiopeia?tab=coc-ov-file)

## License

Three licences, split by what a file is. The example files themselves, meaning mappings, manifests, schemas, and `example.toml`, are [MIT-0](LICENSE-MIT-0.md): copy them into your own pipeline, no attribution required. The prose is [CC-BY-4.0](LICENSE-CC-BY.md) and the runner is [EUPL-1.2](LICENSE-EUPL.md). No dataset is part of this repository; each one is downloaded from its publisher at run time under its own terms, which that example's `example.toml` records. [NOTICE.md](NOTICE.md) has the detail and the trademark terms.

## About

Cassiopeia is part of Vela Tools, an open-core infrastructure project for the NGSI-LD ecosystem. SenLab d.o.o. builds it in Ljubljana, Slovenia. Learn more at [velacontext.com](https://velacontext.com).
