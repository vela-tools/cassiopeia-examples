# Reading SensorThings Things with several Locations

A SensorThings Thing usually has one Location, a GeoJSON point. The standard lets it have several when its place has more than one representation, each in its own encoding (OGC 18-088, section 8.2.1). Taiwan's national air quality stations, as the Civil IoT platform serves them, are such Things: each has a GeoJSON point and, beside it, a Location whose encoding is a postal address. This example writes each station as a `PointOfInterest`. Its `location` comes from the point, and its `address` from the other Location, which the mapping finds by its encoding.

## Get the data

Taiwan's [Civil IoT](https://ci.taiwan.gov.tw/dsp/) platform gathered the country's government sensor networks (air, water, land and disaster monitoring) into SensorThings services on FROST-Server. The project ended on 2025-12-31, and the services moved from `sta.ci.taiwan.gov.tw` to `sta.colife.org.tw`, where the main ones are still served. The datasets are published under Taiwan's [Open Government Data License, version 1.0](https://data.gov.tw/license), as the platform's listing on the [national dataset catalogue](https://ark.nchc.org.tw/en/organization/civiliot) states.

This example reads `https://sta.colife.org.tw/STA_AirQuality_v2/v1.0`, the Ministry of Environment's 77 air quality monitoring stations with 1,386 Datastreams. The service no longer receives readings: none of its Datastreams holds an Observation. Its Things and Locations are still served, and an inventory of the stations needs nothing more. [Example 51](../51-sensorthings-taiwan-end-to-end/example.md) reads the platform's live services.

```bash
cassiopeia sensorthings readings https://sta.colife.org.tw/STA_AirQuality_v2/v1.0
```

```text
reading    unit     unit code  data streams  multi data streams  things with repeats
AQI                 unknown    77            0                   0
CO         ppm      59         77            0                   0
NO2        ppb      61         77            0                   0
O3         ppb      61         77            0                   0
PM10       μg/m3    GQ         77            0                   0
PM2.5      μg/m3    GQ         77            0                   0
SO2        ppb      61         77            0                   0
WindDirec  degrees  unknown    77            0                   0
WindSpeed  m/sec    MTS        77            0                   0
```

The command counts Datastreams without reading their Observations, so every row appears even though no Datastream holds an Observation. The table above is cut: rows such as `CO_8hr`, `PM2.5_AVG` and `Status` are left out. Each station has one Datastream for each of 18 properties.

One station's Thing with its Locations shows the two encodings:

```bash
curl --globoff "https://sta.colife.org.tw/STA_AirQuality_v2/v1.0/Things(1)?\$expand=Locations"
```

## Get the schema

```bash
cassiopeia sdm download
```

## The record shape

The manifest has no `sensorthings` block, so the read is a latest read of every Datastream with the defaults. A Thing whose Datastreams hold no Observation gives a record without `readings`, like the Keelung station's:

```json
{
    "server": {
        "url": "https://sta.colife.org.tw/STA_AirQuality_v2/v1.0",
        "host": "sta.colife.org.tw",
        "version": "v1.0"
    },
    "thing": {
        "id": 1,
        "name": "空氣品質測站-基隆",
        "description": "空氣品質測站-基隆",
        "properties": {
            "city": "基隆市",
            "areaName": "北部空品區",
            "township": "信義區",
            "authority": "環境部",
            "stationID": "EPA001",
            "englishName": "Keelung",
            "stationName": "基隆",
            "stationType": "一般測站"
        }
    },
    "location": {
        "type": "Point",
        "coordinates": [121.7601, 25.1292]
    },
    "locations": [
        {
            "id": 1,
            "name": "空氣品質測站-基隆",
            "description": "空氣品質測站-基隆",
            "encodingType": "application/vnd.geo+json",
            "location": {
                "type": "Point",
                "coordinates": [121.7601, 25.1292]
            }
        },
        {
            "id": 2,
            "name": "空氣品質測站-基隆",
            "description": "空氣品質測站-基隆",
            "encodingType": "address",
            "location": {
                "type": "Address",
                "address": "基隆市信義區東信路324號"
            }
        }
    ]
}
```

`locations` holds every Location of the Thing in id order, each with its members as served. A GeoJSON Location's `location` is its geometry. Any other Location keeps its `location` as the server sent it: here, an object whose `address` is the station's street address. The top-level `location` is the geometry of the Thing's GeoJSON Location with the lowest id. Cassiopeia recognises GeoJSON by `encodingType`, either `application/geo+json` or the older `application/vnd.geo+json` this server uses, so the address Location never becomes the record's `location`, whatever its id. A Thing with Locations but none in GeoJSON would have no `location`, and the run would report it once under `ingest-locations-unreadable`.

The Thing's `properties` describe the station. `stationID` is the ministry's station code, `englishName` and `stationName` are the station's English and Chinese names, `city` and `township` its county and district, `stationType` the kind of station, and `authority` the ministry, 環境部. The Thing's own `name`, 空氣品質測站-基隆, means "air quality station Keelung". The mapping reads these properties, `location` and `locations`, and nothing else.

## Finding a Location by its encoding

```json5
address: {
    transformation: "object",
    mappings: {
        streetAddress: {
            source: "{% for place in locations %}{% if place.encodingType == 'address' %}{{ place.location.address }}{% endif %}{% endfor %}",
        },
        addressLocality: {
            source: "{{ thing.properties.township }}",
        },
        addressRegion: {
            source: "{{ thing.properties.city }}",
        },
        addressCountry: {
            source: "TW",
        },
    },
},
```

The address Location comes second on every station, so `{{ locations[1].location.address }}` would read it. On another Thing, the same index reads whatever comes second, and a server that adds or replaces a Location changes the order. The loop picks the Location whose `encodingType` is `address`, wherever it sits, and gives nothing for a Thing without one. `mappings` builds `address` as an object, the shape Smart Data Models use for a postal address. A member whose template gives nothing is left out of the object.

## A station as a point of interest

These stations have no readings, so an observation model does not fit: `AirQualityObserved` requires a `dateObserved`, and there is none. `PointOfInterest` describes a place by its name, category, location and address, and the server still serves all four:

- `name` is the English name, `Keelung`, which `PointOfInterest` requires, and `alternateName` the Chinese one, `基隆`.
- `description` is the Thing's own name.
- `category` holds the station type as the ministry writes it: 一般測站 (general) at 60 stations, 交通測站 (traffic) at six, and 工業測站 (industrial) at five. The rest are background, park and other stations.
- `dataProvider` is the authority, and `location` the point.
- The identity is the station code, so Keelung's entity is `urn:ngsi-ld:PointOfInterest:taiwan-EPA001`.

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
    --volume "$HOME/.cache/cassiopeia-examples/schemas:/var/lib/cassiopeia/schemas" \
    --workdir /data \
    ghcr.io/vela-tools/cassiopeia:v1.2.0 \
    map \
    --manifest manifest.json5
```

From the repository root, the runner runs the mapping and checks the output in one step:

```bash
cargo run -- run 48
cargo run -- run 48 --runtime docker
```

The server is in Taiwan and takes about a second per request from Europe. The read follows two pages of Datastreams and takes about 20 seconds. It writes 77 entities to `out/PointOfInterest.jsonld`.

## Read the result

```json
{
    "@context": "https://raw.githubusercontent.com/smart-data-models/dataModel.PointOfInterest/master/context.jsonld",
    "id": "urn:ngsi-ld:PointOfInterest:taiwan-EPA001",
    "type": "PointOfInterest",
    "name": {
        "type": "Property",
        "value": "Keelung"
    },
    "alternateName": {
        "type": "Property",
        "value": "基隆"
    },
    "description": {
        "type": "Property",
        "value": "空氣品質測站-基隆"
    },
    "category": {
        "type": "Property",
        "value": [
            "一般測站"
        ]
    },
    "dataProvider": {
        "type": "Property",
        "value": "環境部"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                121.7601,
                25.1292
            ]
        }
    },
    "address": {
        "type": "Property",
        "value": {
            "streetAddress": "基隆市信義區東信路324號",
            "addressLocality": "信義區",
            "addressRegion": "基隆市",
            "addressCountry": "TW"
        }
    }
}
```

`location` is a GeoProperty that holds the GeoJSON geometry (ETSI GS CIM 009 v1.9.1, clause 4.7). `address` is a Property whose value is a JSON object, which is how NGSI-LD carries a structured value that is not a geometry (clause 4.5.2). The Datong traffic station in Taipei, `urn:ngsi-ld:PointOfInterest:taiwan-EPA016`, has the same shape, with `交通測站` as its category and `臺北市大同區重慶北路3段2號` as its street address. If the service receives readings again, the records gain `readings`. The entities stay the same, since the mapping reads none.
