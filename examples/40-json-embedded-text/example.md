# Reading JSON text embedded in a JSON record

The Global Biodiversity Information Facility (GBIF) serves species occurrences as JSON, and one member of each occurrence is not an object but the text of one. This example maps the iNaturalist observations GBIF holds for Slovenia in the first days of January 2026 to `SpeciesObservation` entities, and decodes that text into a `JsonProperty`.

The idea this example adds is that JSON can arrive as a string. The `json` transformation reads the text as the value it encodes, so a nested object that a source flattened into a string comes back as an object in the entity. The example also reads each observation's page and licence as IRIs with the `iri` transformation, which [the previous example](../39-geojson-urn-identifier/example.md) introduces next to `urn`. It builds on [mapping nested JSON collections](../32-json-nested-collections/example.md), whose `mappings` list reads the `results` array here.

## Get the data

GBIF's occurrence search returns a page of up to 300 records. The query selects the iNaturalist research-grade dataset, observations in Slovenia with coordinates, and an event date from 1 to 4 January 2026. GBIF states each occurrence's licence in the record, and the observer chooses it: CC0, CC BY, or CC BY-NC. Of the 126 records in the window, 122 are CC BY-NC.

```bash
curl --fail --location \
    --output data/occurrences.json \
    "https://api.gbif.org/v1/occurrence/search?datasetKey=50c9509d-22c7-4a22-a47d-8c48425ef4a7&country=SI&hasCoordinate=true&eventDate=2026-01-01,2026-01-04&limit=300"
```

Observers add observations for a past date for days afterwards, so a later download can hold more records than quoted here.

## The record shape

The response is an object with paging members, a `results` array of occurrences, and an empty `facets` array. Two array members mean it is no single-array envelope, so the input names `results` as a collection in a `mappings` list. One occurrence looks like this, shortened to the members the mapping reads:

```json
{
    "key": 5938223171,
    "scientificName": "Asparagus acutifolius L.",
    "kingdom": "Plantae",
    "taxonKey": 2768995,
    "eventDate": "2026-01-03T15:30",
    "decimalLatitude": 46.057894,
    "decimalLongitude": 13.499665,
    "coordinateUncertaintyInMeters": 49.0,
    "references": "https://www.inaturalist.org/observations/333340246",
    "license": "http://creativecommons.org/licenses/by/4.0/legalcode",
    "dynamicProperties": "{\"leaves\":[\"colored leaves\",\"green leaves\"]}"
}
```

Look at the last member. `dynamicProperties` is a Darwin Core term for additional facts about a record, and this source fills it with the text of a JSON object, quotes escaped, in a string. The mapping ignores the other members of the record, among them the observer's name.

## JSON in a string

A `JsonProperty` keeps a JSON value as JSON in the entity, in its `json` member (ETSI GS CIM 009 v1.9.1, clause 4.5.24.2), and that member has to be an object or an array of objects. The text in `dynamicProperties` is neither yet. A `JsonProperty` with no transformation keeps text as text and refuses it, so the attribute names `json`:

```json5
evidence: {
    source: "{{ dynamicProperties }}",
    type: "JsonProperty",
    transformation: "json",
},
```

The `json` transformation reads text as whatever JSON value it encodes, keeping any `null` inside it. Text that is not JSON is dropped and the run warns under `extractor-json-text-unreadable`, and JSON that is no object or array of objects is dropped under `extractor-json-payload-misshapen`. A record whose `dynamicProperties` is `null` gives no attribute and no warning. In this window 78 of the 126 records have one.

The text is not uniform. Most records say `{"evidenceOfPresence":"organism"}`, some say `track`, and one lists two kinds of leaves as an array. A `JsonProperty` suits that, because it keeps whatever object the observer's app wrote and never interprets it.

## IRIs

The occurrence's page and licence are absolute URLs, read with the `iri` transformation on a plain `Property`:

```json5
observationPage: {
    source: "{{ references }}",
    type: "Property",
    transformation: "iri",
},
```

The transformation checks that the text is an absolute IRI (RFC 3987) and writes it in its normal form: the scheme and host lowercased, percent-encodings in uppercase hex, and for `http` and `https` the default port dropped and an empty path written as `/`. These values are already in that form, so they are written unchanged. A value that is no IRI is dropped and the run warns under `extractor-iri-unreadable`, and the entity is still written. On a `Property` the result is text.

## Dates and positions

The event date is a local date and time with no zone, written with or without seconds (`2026-01-03T15:30` and `2026-01-04T09:55:36` both occur). A date and time with a `T` but no zone is not read as a `datetime`, so the mapping keeps the date, which every record states in full:

```json5
observationDate: {
    source: "{{ eventDate | split(pat='T') | first }}",
    type: "Property",
    transformation: "date",
},
```

The `date` transformation writes `2026-01-03` with a `valueType` of `Date` (ETSI GS CIM 009 v1.9.1, clause 4.5.2). The position is built from the two decimal members as a `point`, longitude first, because GeoJSON orders its coordinates that way. The uncertainty is present on only some records, and a record without it has no `coordinateUncertainty`.

## Run it

The occurrences sit in a collection of a JSON document, so the run takes a manifest:

```bash
cassiopeia map \
    --manifest manifest.json5
```

The same run in a container mounts this directory at `/data` and makes it the working directory, so the paths do not change. For Podman, replace `docker` with `podman` and drop the `--user` line: rootless Podman already maps the container's root to your user.

```bash
docker run --rm \
    --user "$(id -u):$(id -g)" \
    --volume "$PWD:/data" \
    --workdir /data \
    ghcr.io/vela-tools/cassiopeia:v1.1.1 \
    map \
    --manifest manifest.json5
```

From the repository root, the runner downloads the dataset, runs the mapping, and checks the output in one step:

```bash
cargo run -- run 40
cargo run -- run 40 --runtime docker
```

The run writes `out/SpeciesObservation.json` with one entity per occurrence, 126 at the time of writing. `SpeciesObservation` is a plain data model with no published schema, so every entity is written without a schema check and the run counts that once per entity as `schema-absent`.

## Read the result

The occurrence above becomes this entity. The text that arrived as a quoted string is now an object under `json`:

```json
{
    "id": "urn:ngsi-ld:SpeciesObservation:5938223171",
    "type": "SpeciesObservation",
    "scientificName": {
        "type": "Property",
        "value": "Asparagus acutifolius L."
    },
    "kingdom": {
        "type": "Property",
        "value": "Plantae"
    },
    "taxonKey": {
        "type": "Property",
        "value": 2768995
    },
    "observationDate": {
        "type": "Property",
        "value": "2026-01-03",
        "valueType": "Date"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [13.499665, 46.057894]
        }
    },
    "coordinateUncertainty": {
        "type": "Property",
        "value": 49.0,
        "unitCode": "MTR"
    },
    "observationPage": {
        "type": "Property",
        "value": "https://www.inaturalist.org/observations/333340246"
    },
    "license": {
        "type": "Property",
        "value": "http://creativecommons.org/licenses/by/4.0/legalcode"
    },
    "evidence": {
        "type": "JsonProperty",
        "json": {
            "leaves": [
                "colored leaves",
                "green leaves"
            ]
        }
    }
}
```

`evidence` is a `JsonProperty`, whose `json` member is raw JSON that NGSI-LD never interprets, which suits a free-form record from an observer's app.
