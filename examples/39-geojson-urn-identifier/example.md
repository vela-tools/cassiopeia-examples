# Reading URNs and IRIs from dataset metadata

The World Ozone and Ultraviolet Radiation Data Centre (WOUDC) publishes a register that describes each of its datasets. This example maps the register's eleven records to `Dataset` entities, and reads four values that are identifiers in their own right: a URN, a DOI, a contact page, and a list of vocabulary IRIs.

The idea this example adds is that a value that has to be a URN or an IRI can be read as one. The `urn` and `iri` transformations check the value, write it in its normal form, and drop a value that is neither, with a warning, instead of passing any text through. [Example 26](../26-csv-vocab-property/example.md) reads an IRI into a `VocabProperty`; here the same checks apply to a plain `Property` and to every item of a `ListProperty`.

## Get the data

WOUDC serves its register as a GeoJSON `FeatureCollection` from an OGC API. Every record's `rights` member states the terms: access to the data is unlimited and free of charge for scientific, educational and policy use, and the data providers have to be acknowledged. The register holds eleven datasets, so one request returns all of them.

```bash
curl --fail --location \
    --output data/discovery-metadata.json \
    "https://api.woudc.org/collections/discovery_metadata/items?f=json&limit=100"
```

## The record shape

Each feature is one dataset. Its `id` is a WMO metadata identifier, and the members the mapping reads sit under `properties`. This record is shortened to those members:

```json
{
    "id": "urn:wmo:md:org-woudc:Spectral",
    "type": "Feature",
    "geometry": {
        "type": "Polygon",
        "coordinates": [[[-156.68, -89.9965], [169.684, -89.9965], [169.684, 82.5], [-156.68, 82.5], [-156.68, -89.9965]]]
    },
    "properties": {
        "title": "Spectral",
        "version": "1.0",
        "externalIds": [{"scheme": "https://doi.org", "value": "10.14287/10000011"}],
        "themes": [
            {"scheme": "https://codes.wmo.int/wis/topic-hierarchy/earth-system-discipline", "concepts": [{"id": "atmospheric-composition"}]},
            {"scheme": "https://api.woudc.org/collections/instruments", "concepts": [{"id": "acton"}]}
        ],
        "contacts": [{"links": [{"rel": "canonical", "href": "https://woudc.org/en/contact"}]}]
    }
}
```

The mapping also reads `description`, and ignores the rest. The feature's own members outside `properties`, such as the `links` the source also carries, are not part of a GeoJSON record, so the mapping cannot reach them.

## A URN as a value

`id` is a URN in the `wmo` namespace. Cassiopeia does not use a feature's id as the NGSI-LD entity id on its own, and this one is not wanted there: an entity id is itself a URN, `urn:ngsi-ld:Dataset:...`, so the WMO identifier stays a value and the identity takes its last segment:

```json5
identity: {
    entityName: "{{ id | split(pat=':') | last | replace(from='.', to='-') }}",
},
```

The `replace` matters for two records, `UmkehrN14_1.0` and `UmkehrN14_2.0`. A dot is not kept in an identifier, so without it they would become `UmkehrN14_10` and `UmkehrN14_20`, which read as other levels. As hyphens they are `UmkehrN14_1-0` and `UmkehrN14_2-0`.

The URN itself is kept as a `Property` read through the `urn` transformation:

```json5
metadataIdentifier: {
    source: "{{ id }}",
    type: "Property",
    transformation: "urn",
},
```

The transformation admits only a URN (RFC 8141) and writes it in its normal form: the `urn:` prefix and the namespace identifier lowercased, percent-encodings in uppercase hex, and the rest of the text as it is. `URN:WMO:md:org-woudc:Spectral` would come out as `urn:wmo:md:org-woudc:Spectral`. Text that is no URN is dropped, and the run warns under `extractor-urn-unreadable`. On a `Property` the result is text, so the value is written as `"urn:wmo:md:org-woudc:Spectral"`.

## IRIs as values

A DOI is held as a scheme and a suffix in two members. A template joins them, and the `iri` transformation checks that the result is an absolute IRI (RFC 3987):

```json5
doi: {
    source: "{{ properties.externalIds[0].scheme }}/{{ properties.externalIds[0].value }}",
    type: "Property",
    transformation: "iri",
},
```

This gives `https://doi.org/10.14287/10000011`. Like `urn`, `iri` writes the normal form: the scheme and host lowercased, percent-encodings in uppercase hex, and for `http` and `https` the default port dropped and an empty path written as `/`. Text that is no absolute IRI, a relative reference such as `/a/b` included, is dropped and the run warns under `extractor-iri-unreadable`. The contact page is read the same way.

## A list of IRIs

A record names one or two vocabularies its themes come from, so the list has a different length from record to record. The template gathers the `scheme` of each theme into one text and splits it, and the `iri` transformation on a `ListProperty` then reads every item:

```json5
themeSchemes: {
    source: "{% set_global schemes = '' %}{% for theme in properties.themes %}{% set_global schemes = schemes ~ theme.scheme ~ ' ' %}{% endfor %}{% if schemes != '' %}{{ schemes | trim | split(pat=' ') }}{% endif %}",
    type: "ListProperty",
    transformation: "iri",
},
```

On a `ListProperty`, a transformation says how to read each item. An item the transformation cannot read drops the whole attribute, because a list's order carries meaning and no item may vanish from it unannounced; the entity is still written. The `{% if schemes != '' %}` guard gives a record with no themes no attribute.

## Run it

This is a single GeoJSON file, so the run needs no manifest:

```bash
cassiopeia map \
    --input data/discovery-metadata.json \
    --mapping dataset.json5 \
    --type geojson \
    --output out \
    --context none \
    --validation-mode fail-when-schema
```

The same run in a container mounts this directory at `/data` and makes it the working directory, so the paths do not change. For Podman, replace `docker` with `podman` and drop the `--user` line: rootless Podman already maps the container's root to your user.

```bash
docker run --rm \
    --user "$(id -u):$(id -g)" \
    --volume "$PWD:/data" \
    --workdir /data \
    ghcr.io/vela-tools/cassiopeia:v1.2.0 \
    map \
    --input data/discovery-metadata.json \
    --mapping dataset.json5 \
    --type geojson \
    --output out \
    --context none \
    --validation-mode fail-when-schema
```

From the repository root, the runner downloads the dataset, runs the mapping, and checks the output in one step:

```bash
cargo run -- run 39
cargo run -- run 39 --runtime docker
```

The run writes `out/Dataset.json` with 11 entities. `Dataset` is a plain data model with no published schema, so every entity is written without a schema check and the run counts that once per entity as `schema-absent`.

## Read the result

The first dataset carries all three kinds of identifier. The description and the polygon are shortened:

```json
{
    "id": "urn:ngsi-ld:Dataset:Spectral",
    "type": "Dataset",
    "title": {
        "type": "Property",
        "value": "Spectral"
    },
    "description": {
        "type": "Property",
        "value": "Similar to broad-band, Spectral measurements involves UV rad..."
    },
    "version": {
        "type": "Property",
        "value": "1.0"
    },
    "metadataIdentifier": {
        "type": "Property",
        "value": "urn:wmo:md:org-woudc:Spectral"
    },
    "doi": {
        "type": "Property",
        "value": "https://doi.org/10.14287/10000011"
    },
    "contactPage": {
        "type": "Property",
        "value": "https://woudc.org/en/contact"
    },
    "themeSchemes": {
        "type": "ListProperty",
        "valueList": [
            "https://codes.wmo.int/wis/topic-hierarchy/earth-system-discipline",
            "https://api.woudc.org/collections/instruments"
        ]
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Polygon",
            "coordinates": [[[-156.68, -89.9965], [-156.68, 82.5]]]
        }
    }
}
```

The `valueList` member is how NGSI-LD writes a `ListProperty` (ETSI GS CIM 009 v1.9.1, clause 4.5.21). The `UV Index (hourly maximum)` record names only the first vocabulary, so its list has one item.

One quirk of the source shows in the data. Every record writes its version as text except `UmkehrN14_2-0`, which writes the number `2.0`. A JSON number has no trailing zero, so the `string` transformation reads it as `2`, and that entity's `version` is `"2"` where the others read `"1.0"`.
