# Mapping nested JSON collections

A JSON API often keeps its records several arrays deep, below objects that are not records at all. Cassiopeia can read such a document as **nested collections**: every array of objects is a collection labelled with the member path that leads to it, and a manifest maps the collections it lists. This example maps the London Air Quality Network's site register and its hourly air-quality index, and produces one entity per monitoring site and one per pollutant reading.

The [KML collections example](../17-kml-folder-collections/example.md) routed the folders of one file to their own mappings. Here the collections are JSON arrays, and a record also reads the objects that enclose it, so a pollutant reading can name the site it was measured at.

## Get the data

The [London Air Quality Network](https://www.londonair.org.uk/) is run by the Environmental Research Group at Imperial College London. Its API needs no key, and its documentation puts the data under the Open Government Licence v2, with attribution to the Environmental Research Group and the London Air Quality Network. The site register lists every site the network has run, open or closed, about 260 of them. The index feed holds the latest hourly readings, about 200 of them, one for each pollutant at each reporting site:

```bash
curl --fail --location --output data/sites.json "https://api.erg.ic.ac.uk/AirQuality/Information/MonitoringSites/GroupName=London/Json"
curl --fail --location --output data/index.json "https://api.erg.ic.ac.uk/AirQuality/Hourly/MonitoringIndex/GroupName=London/Json"
```

Both are live: the index changes every hour, and the register changes when a site opens or closes.

## The record shape

The API is a JSON rendering of XML. An element's attributes become members whose names start with `@`, and its child elements become members named after them. The register keeps its sites two levels down, at `Sites.Site`:

```json
{
    "Sites": {
        "Site": [
            {
                "@LocalAuthorityCode": "1",
                "@LocalAuthorityName": "Barking and Dagenham",
                "@SiteCode": "BG1",
                "@SiteName": "Barking and Dagenham - Rush Green",
                "@SiteType": "Suburban",
                "@DateClosed": "",
                "@DateOpened": "1999-11-02 00:00:00",
                "@Latitude": "51.563752",
                "@Longitude": "0.177891",
                "@LatitudeWGS84": "6721627.34498",
                "@LongitudeWGS84": "19802.7355367",
                "@DisplayOffsetX": "0",
                "@DisplayOffsetY": "0",
                "@DataOwner": "Barking and Dagenham",
                "@DataManager": "King's College London",
                "@SiteLink": "http://www.londonair.org.uk/london/asp/publicdetails.asp?site=BG1"
            }
        ]
    }
}
```

The site mapping reads `@SiteCode`, `@SiteName`, `@SiteType`, `@LocalAuthorityName`, `@DateOpened`, `@DateClosed`, `@Latitude`, and `@Longitude`, and ignores the rest. Despite their names, `@LatitudeWGS84` and `@LongitudeWGS84` hold Web Mercator metres, so the mapping takes its point from the plain `@Latitude` and `@Longitude` degrees.

The index nests three levels deep: local authorities hold sites, and sites hold one `Species` entry per pollutant. The root has no array member at all, so no top-level rule could reach a reading:

```json
{
    "HourlyAirQualityIndex": {
        "@GroupName": "London",
        "@TimeToLive": "33",
        "LocalAuthority": [
            {
                "@LocalAuthorityCode": "8",
                "@LocalAuthorityName": "Croydon",
                "Site": [
                    {
                        "@BulletinDate": "2026-11-05 06:00:00",
                        "@SiteCode": "CR5",
                        "@SiteName": "Croydon - Norbury",
                        "@SiteType": "Kerbside",
                        "@Latitude": "51.411349",
                        "@Longitude": "-0.12311",
                        "Species": {
                            "@SpeciesCode": "NO2",
                            "@SpeciesDescription": "Nitrogen Dioxide",
                            "@AirQualityIndex": "1",
                            "@AirQualityBand": "Low",
                            "@IndexSource": "Measurement"
                        }
                    }
                ]
            }
        ]
    }
}
```

This excerpt keeps one of Croydon's three sites, and leaves out some members of the authority and the site. A reading carries its pollutant (`@SpeciesCode`, `@SpeciesDescription`), its index on the UK Daily Air Quality Index scale of 1 to 10, and its band. The site carries the code, the position, and the time of the bulletin. The reading mapping reads the reading's members and the site's `@SiteCode`, `@BulletinDate`, `@Latitude`, and `@Longitude`, plus the authority's `@LocalAuthorityName`, and ignores the rest.

CR5 measures only nitrogen dioxide, so its `Species` is an object, not an array. XML-to-JSON conversion writes an element that occurs once as an object and the same element occurring twice as an array. Croydon's `Site` is an array because the borough has three sites, and a borough with one site has a `Site` object instead.

## Listing the collections

Each input lists the collections it maps in `mappings`, by their member path without array indices:

```json5
{
    version: "v1",
    inputs: [
        {
            source: "data/sites.json",
            format: "json",
            mappings: [
                {
                    collection: "Sites.Site",
                    mapping: "site.json5",
                },
            ],
        },
        {
            source: "data/index.json",
            format: "json",
            mappings: [
                {
                    collection: "HourlyAirQualityIndex.LocalAuthority.Site.Species",
                    mapping: "reading.json5",
                },
            ],
        },
    ],
}
```

Every item of a listed collection becomes one record. A collection that is not listed yields no records, so the local authorities and the sites of the index feed become no entities of their own. A lone object on a listed path counts as a collection of one item, so CR5's single reading and the readings of a single-site borough are read like all the others.

An input with a single `mapping` keeps reading JSON by its top-level shape, as every earlier JSON example does. Only a `mappings` list reads the document as nested collections.

## What a record encloses

A record keeps its own members, and also carries every object that encloses it on the path from the root, under the name of the member that holds it. CR5's nitrogen dioxide reading becomes this record:

```json
{
    "@SpeciesCode": "NO2",
    "@SpeciesDescription": "Nitrogen Dioxide",
    "@AirQualityIndex": "1",
    "@AirQualityBand": "Low",
    "@IndexSource": "Measurement",
    "Site": {
        "@BulletinDate": "2026-11-05 06:00:00",
        "@SiteCode": "CR5",
        "@SiteName": "Croydon - Norbury",
        "@SiteType": "Kerbside",
        "@Latitude": "51.411349",
        "@Longitude": "-0.12311",
        "@LatitudeWGS84": "6694381.70052",
        "@LongitudeWGS84": "-13704.5425116",
        "@OwnerID": "8"
    },
    "LocalAuthority": {
        "@LocalAuthorityCode": "8",
        "@LocalAuthorityName": "Croydon",
        "@LaCentreLatitude": "51.372361",
        "@LaCentreLongitude": "-0.100401",
        "@LaCentreLatitudeWGS84": "6687426.268766",
        "@LaCentreLongitudeWGS84": "-11176.588195"
    },
    "HourlyAirQualityIndex": {
        "@GroupName": "London",
        "@TimeToLive": "33"
    }
}
```

An enclosing object comes without its collections and without the member the path continues through, so `Site` holds no `Species`, and `LocalAuthority` holds no `Site`. When a name clashes, the record's own member wins, then the nearest enclosing object.

The reading mapping names an entity after its site and its pollutant, so readings of the same pollutant at different sites stay apart:

```json5
identity: {
    entityName: "{{ Site['@SiteCode'] }}-{{ this['@SpeciesCode'] }}",
},
```

`@` is not a valid template identifier, so every member is read with bracket notation: through `this` for the record's own members, and through the enclosing object's name for the others. The same site code links the reading to its site entity from the register:

```json5
refAirQualityMonitoringSite: {
    source: "{{ Site['@SiteCode'] }}",
    type: "Relationship",
    target: {
        entity: "AirQualityMonitoringSite",
    },
},
```

## Readings without data

The index lists every pollutant a site is equipped for, including the ones it did not report this hour. Those carry the band `No data` and an index of `0`, which is no value on a scale that starts at 1. The mapping keeps the band and leaves the index out behind a guard, so a reading without data has no `airQualityIndex` at all:

```json5
airQualityIndex: {
    source: "{% if this['@AirQualityBand'] != 'No data' %}{{ this['@AirQualityIndex'] }}{% endif %}",
    type: "Property",
    transformation: "integer",
},
```

The bulletin time is London local time with no offset. Cassiopeia reads a space-separated time without an offset as UTC, which is right in winter and an hour off in summer. The mapping writes it as an RFC 9557 time with a bracketed zone, so the conversion follows British Summer Time:

```json5
dateObserved: {
    source: "{{ Site['@BulletinDate'] | replace(from=' ', to='T') }}[Europe/London]",
    type: "Property",
    transformation: "datetime",
},
```

The register writes a site's opening and closing as midnight, so the site mapping keeps only the day with a `date` transformation. A site that is still open has an empty `@DateClosed`, and a guard leaves it out instead of failing to read it as a date.

## Run it

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

From the repository root, the runner downloads both datasets, runs the mapping, and checks the output in one step:

```bash
cargo run -- run 32
cargo run -- run 32 --runtime docker
```

The run writes one file per entity type. The counts follow the feeds and change from hour to hour:

```text
AirQualityMonitoringSite.json   258 entities
AirQualityIndexObserved.json    194 entities
```

The index sometimes lists one pollutant twice for a site. Both entries name the same entity, so a run can write a few entities fewer than the feed has readings.

## Read the result

The site from the register has no `dateClosed`, because it is still running. `dateOpened` carries a `valueType` of `Date`, so a broker reads it as a date rather than as text (ETSI GS CIM 009 v1.9.1, clause 4.5.2):

```json
{
    "id": "urn:ngsi-ld:AirQualityMonitoringSite:BG1",
    "type": "AirQualityMonitoringSite",
    "name": {
        "type": "Property",
        "value": "Barking and Dagenham - Rush Green"
    },
    "siteType": {
        "type": "Property",
        "value": "Suburban"
    },
    "localAuthority": {
        "type": "Property",
        "value": "Barking and Dagenham"
    },
    "dateOpened": {
        "type": "Property",
        "value": "1999-11-02",
        "valueType": "Date"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                0.177891,
                51.563752
            ]
        }
    }
}
```

CR5's reading came from a lone `Species` object, and still has its site, its borough, and its position, all read from the objects that enclose it:

```json
{
    "id": "urn:ngsi-ld:AirQualityIndexObserved:CR5-NO2",
    "type": "AirQualityIndexObserved",
    "pollutant": {
        "type": "Property",
        "value": "NO2"
    },
    "pollutantName": {
        "type": "Property",
        "value": "Nitrogen Dioxide"
    },
    "airQualityBand": {
        "type": "Property",
        "value": "Low"
    },
    "airQualityIndex": {
        "type": "Property",
        "value": 1
    },
    "dateObserved": {
        "type": "Property",
        "value": "2026-11-05T06:00:00Z",
        "valueType": "DateTime"
    },
    "localAuthority": {
        "type": "Property",
        "value": "Croydon"
    },
    "refAirQualityMonitoringSite": {
        "type": "Relationship",
        "object": "urn:ngsi-ld:AirQualityMonitoringSite:CR5",
        "objectType": "AirQualityMonitoringSite"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                -0.12311,
                51.411349
            ]
        }
    }
}
```

BG1 did not report nitrogen dioxide this hour, so its reading has a band of `No data` and no `airQualityIndex`:

```json
{
    "id": "urn:ngsi-ld:AirQualityIndexObserved:BG1-NO2",
    "type": "AirQualityIndexObserved",
    "pollutant": {
        "type": "Property",
        "value": "NO2"
    },
    "pollutantName": {
        "type": "Property",
        "value": "Nitrogen Dioxide"
    },
    "airQualityBand": {
        "type": "Property",
        "value": "No data"
    },
    "dateObserved": {
        "type": "Property",
        "value": "2026-11-05T06:00:00Z",
        "valueType": "DateTime"
    },
    "localAuthority": {
        "type": "Property",
        "value": "Barking and Dagenham"
    },
    "refAirQualityMonitoringSite": {
        "type": "Relationship",
        "object": "urn:ngsi-ld:AirQualityMonitoringSite:BG1",
        "objectType": "AirQualityMonitoringSite"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                0.177891,
                51.563752
            ]
        }
    }
}
```

Two nested documents in, two linked entity types out, with no reshaping step in between.
