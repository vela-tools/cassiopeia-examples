# Reading authenticated and irregular CSV sources

This example reads the hourly nitrogen dioxide readings of two European capitals, maps both to `AirQualityObserved`, upserts them to a local broker, and queries the two cities as one network. The sources are awkward in different ways. Brussels publishes through an API that takes a key, and Vienna publishes a table with a title row above it and a header that spans three rows. Both are CSV, and neither reads as plain RFC 4180. The manifest handles both with three options an input can carry: `headers`, which can read a credential from the environment, `dialect`, and the automatic retry of a source that is busy or failing. Detection does the rest. [Example 23](../23-json-scheduling/example.md) introduced a manifest that fetches a remote source and delivers to a broker. [Example 6](../06-csv-messy-headers/example.md) read headers that are awkward to address, and this example reads one that is awkward to find.

## Get the data

The Brussels-Capital Region's hourly NO2 means come from the Belgian Interregional Environment Agency (IRCEL-CELINE), republished by the City of Brussels on its [Opendatasoft portal](https://opendata.brussels.be/explore/dataset/real-time-air-quality-nitrogen-dioxide-no2/) under CC BY 4.0. The export covers the last 48 hours of 17 stations, one row per station and hour. An hour whose measurement failed is kept with `status` set to `error` and an empty value.

The export also answers without a key. The manifest sends one anyway, since sending a credential is what this example shows, so the run needs one. A key's requests count against a daily quota of its own, 10,000 requests, which every response reports in its `X-RateLimit-*` headers. To get a key, register at [opendata.brussels.be](https://opendata.brussels.be/signup/); the form asks for a message to the portal's administrator saying what you will use the data for. Then open your account page from your user name in the top right corner, generate a key on the **API keys** tab, and put it in the environment of the shell that runs the example:

```bash
export BRUSSELS_ODS_APIKEY='your-key'
```

To look at the export, ask for it the way the manifest does. The query string keeps five columns and only the valid hours:

```bash
curl -sS --fail -H "Authorization: Apikey $BRUSSELS_ODS_APIKEY" -o brussels.csv 'https://opendata.brussels.be/api/explore/v2.1/catalog/datasets/real-time-air-quality-nitrogen-dioxide-no2/exports/csv?select=ab_eoi_code,ab_name,timestamp,value,geo_point&where=status%3D%22valid%22'
```

Vienna's environmental protection department (MA 22) publishes the current readings of the city's 19 measuring stations as one [CSV file](https://www.wien.gv.at/umwelt/luft-messwerte), replaced every half hour, under CC BY 4.0. The table has one row per station and does not say where the stations are. The city's station layer, served as GeoJSON by its WFS under the same licence, gives each station's location and full name:

```bash
curl -sS --fail -o lumesakt.csv https://www.wien.gv.at/ma22-lgb/umweltgut/lumesakt-v2.csv
curl -sS --fail -o stations.geojson 'https://data.wien.gv.at/daten/geo?service=WFS&request=GetFeature&version=1.1.0&srsName=EPSG:4326&outputFormat=json&typeName=ogdwien:LUFTGUETENETZOGD'
```

The manifest fetches all three sources itself on every run, so these downloads are only for reading.

## Get the schema

`AirQualityObserved` is a published Smart Data Model. Download the catalog once, so both validation and the model's `@context` can be resolved:

```bash
cassiopeia sdm download
```

## The record shape

The Brussels export starts with a UTF-8 byte order mark, ends its lines with CRLF, and separates fields with semicolons:

```text
ab_eoi_code;ab_name;timestamp;value;geo_point
BETB006;41B006 - Bruxelles (Parlement UE);2026-11-11T10:00:00+00:00;16;50.83863118763408, 4.374388283352737
BETB006;41B006 - Bruxelles (Parlement UE);2026-11-11T06:00:00+00:00;18;50.83863118763408, 4.374388283352737
```

`ab_eoi_code` is the station's code in the European air quality network, `value` the hourly mean in µg/m³, and `geo_point` the station's latitude and longitude, separated by a comma and a space.

The Vienna table, shown here converted to UTF-8 and cut to its first eleven columns, is a different kind of file:

```text
Lumes;v2.10;11.11.26-09:30:00
;Zeit-LTM;LTM;Zeit-Wind;WG;WR;Zeit-RF;RF;Zeit-NO2;NO2;Zeit-NOX
;;HMW;;HMW;HMW;;HMW;;HMW;
;MESZ;°C;MESZ;km/h;°;MESZ;%;MESZ;µg/m³;MESZ
STEF;11.11.2026, 09:30;NE;11.11.2026, 09:30;NE;NE;11.11.2026, 09:30;NE;11.11.2026, 09:30;31,94;11.11.2026, 09:30
AKA;11.11.2026, 09:30;4,94;11.11.2026, 09:30;12,75;149,31;11.11.2026, 09:30;84,21;11.11.2026, 09:30;NE;11.11.2026, 09:30
```

The first line is a title: the feed's name, its version, and when it was written. Below it, three rows name each column together: the measurand (`NO2`), the averaging period (`HMW`, the half-hour mean), and the unit (`µg/m³`). Each measurand has a time column of its own (`Zeit-NO2`) beside it. The first column holds the station code and has no header at all. Decimals are written with a comma, and a station that does not measure something writes `NE`, or `---` when the instrument is down. The file is in Windows-1252.

The station layer is GeoJSON. The mapping reads each feature's `geometry` and two of its `properties`, `NAME_KURZ`, the code the table uses, and `NAME`. All three mappings ignore every other column and property.

## Reading the key from the environment

The Brussels input carries the key in a request header. A manifest never has to hold the key itself: `${NAME}` reads a variable from the environment Cassiopeia runs in.

```json5
headers: {
    Authorization: "Apikey ${BRUSSELS_ODS_APIKEY}",
},
```

Opendatasoft reads an API key from the `Authorization` header under its `Apikey` scheme. Cassiopeia sends the input's headers with every request for that input, and with no other request: not to the broker, not to Vienna, and not to the server the `@context` or a schema comes from. A header value is treated as a secret and is left out of debug output.

`${NAME}` works only where a credential belongs: an input's `source` and `headers`, and the output's `url` and `headers`. A mapping cannot read one, so an environment value never ends up in an entity. A variable that is not set stops the run while the manifest loads, before any source is fetched:

```text
× Failed to parse the manifest at 'manifest.json5'
  ╰─ the input header 'authorization': the environment variable 'BRUSSELS_ODS_APIKEY' is not set
```

A key Opendatasoft does not recognise is refused with `401 Unauthorized`, and the run fails on the spot, without showing the key:

```text
× 'https://opendata.brussels.be/api/explore/v2.1/catalog/datasets/real-time-air-quality-nitrogen-dioxide-no2/exports/csv?select=ab_eoi_code,ab_name,timestamp,value,geo_point&where=status%3D%22valid%22' answered 401 Unauthorized
```

## Retried requests

Nothing in the manifest asks for retries, because a remote source is retried by default. A source that answers `429 Too Many Requests` or a `5xx` is asked again, up to three attempts in all, one second and then two seconds apart. When the answer carries a `Retry-After` header (RFC 9110, section 10.2.3), the next attempt waits at least that long. Opendatasoft reports how much of a key's quota is left in its `X-RateLimit-Remaining` header and answers `429` once the quota is spent, so a run that lands on a busy moment rides it out instead of failing.

Any other `4xx`, such as the `401` above, fails the input at once. A wrong URL, a missing key, or a refused request does not change when it is asked again. Only when every attempt is spent does the input fail, with the source's last answer, and the run with it, as `onFailure` decides. [Example 23](../23-json-scheduling/example.md) shows the policy for a schedule, where retrying a whole run is a different setting.

## Stating a dialect in full

A CSV input can describe its file in a `dialect`. The Brussels input states every structural member:

```json5
dialect: {
    delimiter: ";",
    quote: "\"",
    skipRows: 0,
    headerRows: 1,
    terminator: "crlf",
    encoding: "utf-8",
},
```

When `delimiter`, `quote`, `skipRows`, `headerRows`, `terminator`, and `encoding` are all stated, Cassiopeia detects nothing, so the run does not depend on what detection makes of the export on a given day. That matters more than it seems: the delimiter is a parameter of the export, not a property of the data, and `geo_point` holds a comma in every row, so a semicolon is the only delimiter this file can be read with. The byte order mark would win over any stated `encoding`, so the two agree here by construction.

## A title row and a stacked header

The Vienna input states only what no file can reveal:

```json5
dialect: {
    encoding: "windows-1252",
    decimalSeparator: ",",
    nullValues: ["NE", "---"],
},
```

Everything else is detected. `cassiopeia profile` shows what detection finds in the downloaded copy, in the member names a `dialect` takes:

```bash
cassiopeia profile lumesakt.csv
```

```text
lumesakt.csv · 5.0 KiB
CSV · text/csv · 90% confidence
delimiter ";" · quote "\"" · skipRows 1 · headerRows 3 · terminator "lf" · encoding "ISO-8859-2"
```

`skipRows 1` is the title. It has 3 fields where the table has 26, and a row of another width above a table is skipped as its preamble. Rows are counted as CSV records, so a quoted title that spans two lines would still count as one.

`headerRows 3` is the stacked header. A row directly below the header continues it when each of its non-blank cells is a short label rather than a number or a date, and when it leaves the first column blank while every data row fills it. Each column's header cells are then joined from top to bottom with a space. The NO2 column becomes the field `NO2 HMW µg/m³` and its time column `Zeit-NO2 MESZ`. Two columns that share a measurand but differ further down, such as `PM10 MW24 µg/m³` and `PM10 HMW µg/m³`, become two fields. The first column has no header cell at all, so it is keyed by its zero-based position and read as `{{ this['0'] }}`.

The encoding is the one thing detection gets wrong on the downloaded copy. Windows-1252 and ISO-8859-2 differ only in letters such as `µ` and `³`, which ISO-8859-2 reads as `ľ` and `ł`, and nothing in the bytes tells them apart. Fetched by URL, Cassiopeia weighs the source's domain, and a file from a `.at` domain reads as Windows-1252, but a stated `encoding` settles it either way. `decimalSeparator` and `nullValues` are never detected, since nothing in a file tells a decimal comma or a missing-value token apart from text. Without them, `31,94` and `NE` would both stay strings.

## Mapping the two tables

The Brussels export has 48 rows for each station, so the identity is the station code and not anything about the row:

```json5
identity: {
    entityName: "brussels-{{ ab_eoi_code }}",
},
```

Every hour of one station resolves to the same entity. The `observedAt` on `no2` ranks those records, and the latest is kept. `geo_point` is latitude first, and GeoJSON puts longitude first (RFC 7946, section 3.1.1), so the mapping splits the pair and swaps it:

```json5
location: {
    type: "GeoProperty",
    source: "{% set p = geo_point | split(pat=', ') %}{\"type\": \"Point\", \"coordinates\": [{{ p[1] }}, {{ p[0] }}]}",
},
```

The Vienna table writes its times as `11.11.2026, 09:30`, in Vienna local time whatever the `MESZ` label says, and writes midnight as `24:00` of the day before. The mapping rebuilds such a time as `2026-11-11T09:30:00[Europe/Vienna]`, and the `datetime` transformation converts it to UTC with the zone's daylight saving rules:

```json5
dateObserved: {
    source: "{% set p = this['Zeit-NO2 MESZ'] | split(pat=', ') %}{% set d = p[0] | split(pat='.') %}{% if p[1] == '24:00' %}{{ (d[2] ~ '-' ~ d[1] ~ '-' ~ d[0] ~ 'T00:00:00Z') | date_subtract_seconds(seconds=-86400, format='%Y-%m-%dT%H:%M:%S') }}{% else %}{{ d[2] }}-{{ d[1] }}-{{ d[0] }}T{{ p[1] }}:00{% endif %}[Europe/Vienna]",
    transformation: "datetime",
},
```

The reading itself is a field name with spaces in it, read with bracket notation. A station that does not measure NO2 holds `NE`, which the dialect has already turned into a missing value, so the attribute is left out of that station's entity:

```json5
no2: {
    source: "{% set v = this['NO2 HMW µg/m³'] %}{% if v is number and v >= 0 %}{{ v }}{% endif %}",
    transformation: "float",
},
```

The station layer has a mapping of its own, [vienna-station.json5](vienna-station.json5). Its identity is `vienna-{{ properties.NAME_KURZ }}`, the same identifier the table's mapping builds from the code in its first column, so the location and name of a station merge into the same entity as its reading.

## The manifest

[manifest.json5](manifest.json5) binds each source to its mapping, and delivers everything to one broker:

```json5
output: {
    target: "context-broker",
    url: "http://localhost:1056/",
    operation: "upsert",
    context: "default",
    validation: {
        mode: "fail-when-schema",
    },
},
```

`upsert` keeps one entity per station and replaces its reading on every run. `AirQualityObserved` is a published model, so `context: "default"` resolves its `@context` from the downloaded catalog, and `fail-when-schema` stops the run if an entity breaks the model's schema.

## Start the broker

The [docker-compose.yml](docker-compose.yml) starts [Andromeda](https://github.com/mskvarc/andromeda), an NGSI-LD broker backed by PostgreSQL. It serves the API on port 1056, the address the manifest sends to. Start it, and wait for both containers to report healthy:

```bash
docker compose up -d --wait
```

## Run it

With `BRUSSELS_ODS_APIKEY` exported in the same shell:

```bash
cassiopeia map \
    --manifest manifest.json5
```

The same run in a container mounts this directory at `/data` and makes it the working directory, so the paths do not change. For Podman, replace `docker` with `podman` and drop the `--user` line: rootless Podman already maps the container's root to your user. `--network host` lets the container reach the broker at `localhost`. `--env BRUSSELS_ODS_APIKEY` names the variable without a value, which hands the container the value from your shell, so the key never appears on the command line.

```bash
docker run --rm \
    --user "$(id -u):$(id -g)" \
    --network host \
    --env BRUSSELS_ODS_APIKEY \
    --volume "$PWD:/data" \
    --volume "$HOME/.cache/cassiopeia-examples/schemas:/var/lib/cassiopeia/schemas" \
    --workdir /data \
    ghcr.io/vela-tools/cassiopeia:v1.1.1 \
    map \
    --manifest manifest.json5
```

From the repository root, the runner starts the broker, runs the mapping, and checks what the broker holds in one step. It reads `BRUSSELS_ODS_APIKEY` from its own environment:

```bash
cargo run -- run 38
cargo run -- run 38 --runtime docker
```

The run fetches the three sources, maps 673 records into 33 entities, 14 from Brussels and 19 from Vienna, validates them, and upserts them. Three Brussels stations had no valid hour in the export when this was written, so they have no entity. The counts move with the feeds.

## Query the broker

Every query below sends the model's `@context` in a `Link` header, so the broker expands the short names `AirQualityObserved` and `no2` the way the run wrote them (ETSI GS CIM 009 v1.9.1, clause 5.5.7):

```bash
CONTEXT='<https://raw.githubusercontent.com/smart-data-models/dataModel.Environment/master/context.jsonld>; rel="http://www.w3.org/ns/json-ld#context"; type="application/ld+json"'
```

The broker holds both cities under one type. `count=true` asks for the total in the `NGSILD-Results-Count` header, and `limit=0` for no entities:

```bash
curl -sS -G -D - -o /dev/null -H "Link: $CONTEXT" http://localhost:1056/ngsi-ld/v1/entities \
    --data-urlencode type=AirQualityObserved --data-urlencode count=true --data-urlencode limit=0
```

```text
NGSILD-Results-Count: 33
```

A `q` filter compares an attribute's value across both cities at once (clause 4.9). `options=keyValues` returns the simplified representation, and `attrs` keeps the two attributes worth reading:

```bash
curl -sS -G -H "Link: $CONTEXT" http://localhost:1056/ngsi-ld/v1/entities \
    --data-urlencode type=AirQualityObserved --data-urlencode 'q=no2>30' \
    --data-urlencode options=keyValues --data-urlencode attrs=name,no2
```

```json
[
    { "id": "urn:ngsi-ld:AirQualityObserved:vienna-FLO", "type": "AirQualityObserved", "name": "Gerichtsgasse", "no2": 30.84 },
    { "id": "urn:ngsi-ld:AirQualityObserved:vienna-LERG", "type": "AirQualityObserved", "name": "Lerchenfelder Gürtel", "no2": 32.21 },
    { "id": "urn:ngsi-ld:AirQualityObserved:vienna-MBA", "type": "AirQualityObserved", "name": "Hietzinger Kai", "no2": 42.3 }
]
```

A geo-query works on the `location` each city's source supplied in its own way: a GeoJSON geometry in Vienna, a coordinate pair in Brussels (clause 4.10). This one finds every station within 3 km of Stephansplatz:

```bash
curl -sS -G -H "Link: $CONTEXT" http://localhost:1056/ngsi-ld/v1/entities \
    --data-urlencode type=AirQualityObserved --data-urlencode geometry=Point \
    --data-urlencode 'coordinates=[16.3725,48.2084]' --data-urlencode 'georel=near;maxDistance==3000' \
    --data-urlencode options=keyValues --data-urlencode attrs=name,no2
```

```json
[
    { "id": "urn:ngsi-ld:AirQualityObserved:vienna-AKA", "type": "AirQualityObserved", "name": "AKH-Dach" },
    { "id": "urn:ngsi-ld:AirQualityObserved:vienna-AKC", "type": "AirQualityObserved", "name": "AKH" },
    { "id": "urn:ngsi-ld:AirQualityObserved:vienna-LERG", "type": "AirQualityObserved", "name": "Lerchenfelder Gürtel", "no2": 32.21 },
    { "id": "urn:ngsi-ld:AirQualityObserved:vienna-STEF", "type": "AirQualityObserved", "name": "Stephansplatz", "no2": 17.76 },
    { "id": "urn:ngsi-ld:AirQualityObserved:vienna-TAB", "type": "AirQualityObserved", "name": "Taborstraße" }
]
```

Three of the five carry no `no2`, because those stations do not measure it. The two kinds of filter combine. This one asks for the Brussels stations, those inside a polygon around the region, that read 20 µg/m³ or more:

```bash
curl -sS -G -H "Link: $CONTEXT" http://localhost:1056/ngsi-ld/v1/entities \
    --data-urlencode type=AirQualityObserved --data-urlencode geometry=Polygon \
    --data-urlencode 'coordinates=[[[4.24,50.76],[4.48,50.76],[4.48,50.92],[4.24,50.92],[4.24,50.76]]]' \
    --data-urlencode georel=within --data-urlencode 'q=no2>=20' \
    --data-urlencode options=keyValues --data-urlencode attrs=name,no2
```

The readings in these answers are from the moment this page was written. The broker answers with whatever the last run delivered.

When you are done, stop the broker and discard its data:

```bash
docker compose down -v
```

## Read the result

Retrieving one entity by its identifier returns it in the normalized representation. Stephansplatz has its reading from the table and its location and name from the station layer, merged under one identifier:

```json
{
    "id": "urn:ngsi-ld:AirQualityObserved:vienna-STEF",
    "type": "AirQualityObserved",
    "dateObserved": {
        "type": "Property",
        "value": "2026-11-11T09:30:00Z",
        "valueType": "DateTime"
    },
    "name": {
        "type": "Property",
        "value": "Stephansplatz"
    },
    "no2": {
        "type": "Property",
        "value": 17.76,
        "unitCode": "GQ",
        "observedAt": "2026-11-11T09:30:00Z"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                16.37325576,
                48.20815324
            ]
        }
    }
}
```

The table's `11.11.2026, 10:30` became `09:30:00Z`: Vienna is an hour ahead of UTC in November. `valueType` marks `dateObserved` as a `DateTime` (clause 4.5.2), so a query with a time operand compares it as a time and not as text. `GQ` is the UN/CEFACT Common Code for microgram per cubic metre, and `observedAt` stamps the reading with the instant it was measured (clause 4.8).

The Brussels entity has the same shape from a different source. Its location came from one text column and its name from another, and of the station's 48 hours only the newest is left:

```json
{
    "id": "urn:ngsi-ld:AirQualityObserved:brussels-BETB006",
    "type": "AirQualityObserved",
    "dateObserved": {
        "type": "Property",
        "value": "2026-11-11T10:00:00Z",
        "valueType": "DateTime"
    },
    "name": {
        "type": "Property",
        "value": "41B006 - Bruxelles (Parlement UE)"
    },
    "no2": {
        "type": "Property",
        "value": 16,
        "unitCode": "GQ",
        "observedAt": "2026-11-11T10:00:00Z"
    },
    "location": {
        "type": "GeoProperty",
        "value": {
            "type": "Point",
            "coordinates": [
                4.374388283352737,
                50.83863118763408
            ]
        }
    }
}
```

[Output](https://vela-tools.github.io/cassiopeia/docs/guides/output) covers the broker operations, and the [manifest guide](https://vela-tools.github.io/cassiopeia/docs/guides/manifest) every input option this example uses.
