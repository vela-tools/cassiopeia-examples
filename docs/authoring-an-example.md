# Authoring an example

An example is a complete run against a real dataset, written so a reader can follow it from an empty directory to inspected output. New examples follow the layout below from their first commit.

## The directory

Three directories carry the repository, and an example touches only the first:

```text
examples/NN-name/    One example, ordered by its numeric prefix.
docs/                How to run an example, and how to author one.
tools/runner/        The runner: fetch, run, verify, check, index.
```

One example is one directory under `examples/`, named `NN-source-format-technique`: `NN` is a two-digit ordinal, the middle names the source format, and the rest names the technique the example teaches. `07-csv-observed-at`, `15-shapefile-enum-decoding`, `24-json-list-property`. The ordinal fixes the reading order and says nothing about category. Each example builds on the ones before it, so a new one takes the next free number unless it genuinely belongs earlier.

```text
examples/24-json-list-property/
    example.md          Required. The walkthrough.
    example.toml        Required. Datasets, command, and expected output.
    counter.json5       The mapping, named after the model it produces.
    manifest.json5      Present when the example uses a manifest.
    *.schema.json       Present when the example validates against a local schema.
    *.jsonld            Present when the example defines its own @context.
    *.awk               Present when a preparation step reshapes the download.
    docker-compose.yml  Present when the example delivers to a broker.
```

Everything else is generated. `data/` holds the downloaded dataset and `out/` holds the entities; git ignores both. Never commit either, and never commit a dataset in any other form. Filenames are kebab-case.

An example directory holds no run script: the command lives in the descriptor, and the runner executes it. A filter is a different thing. Where a download has to be reshaped before Cassiopeia can read it, a `[[preparation]]` block names the command that does the reshaping, and a filter too long for one line goes beside the mapping in a file of its own, as `flatten.awk` does in example 22.

## example.toml

Everything about an example that is not prose lives in the descriptor: what it teaches, what it downloads, what it runs, and what the run has to produce. The runner reads it, the documentation site reads it, and the page's command blocks come out of it.

```toml
# What the example teaches, and where the documentation site places it.
[example]
# One line describing the example. Must match the H1 in example.md.
title = "Mapping JSON arrays with a ListProperty"
# Two or three words for the documentation sidebar.
label = "ListProperty"
# One sentence for the README index, phrased as what the example shows.
summary = "Store hourly counts in an ordered ListProperty."
# Source formats the example ingests.
formats = ["json"]
# NGSI-LD models the run produces.
models = ["TrafficFlowObserved"]

[requirements]
# The run validates against a published Smart Data Model, so it needs the schema catalog. A
# container run then mounts a host cache at /var/lib/cassiopeia/schemas.
smart-data-models = false

# Arguments passed to cassiopeia, starting with the subcommand. Paths are relative to the example
# directory, because the container runtime mounts that directory and nothing above it: an absolute
# host path does not exist inside the container.
[run]
args = [
    "map",
    "--input",
    "data/counter.json",
    "--mapping",
    "counter.json5",
    "--output",
    "out",
]

# One block per file the example downloads.
[[datasets]]
file = "data/counter.json"
url = "https://example.org/counters.json"
source = "The publishing organisation or repository"
license = "ODbL-1.0"
# "frozen" for a source the publisher has finished with, "live" for one that still grows or is still
# corrected. Required, with no default: only the author knows which it is.
volatility = "live"
records = 8760

# One block per thing the run produces. The counts and identifiers are assertions: `cargo run -- verify`
# reads what came out and fails when it does not hold what the page says it does.
[[outputs]]
file = "out/TrafficFlowObserved.json"
entities = 24
contains = ["urn:ngsi-ld:TrafficFlowObserved:counter-1"]
```

Keys are kebab-case, and nothing sits at the root of the file: every key belongs to a block. `[example]` is required, with its `title`, `label`, `summary`, `formats`, and `models`, and so is `[run]`. Give every downloaded file a `[[datasets]]` block, and everything the run produces an `[[outputs]]` block. `[requirements]` can go when the defaults shown above hold. A download that is not yet in a shape Cassiopeia reads takes a `[[preparation]]` block as well: a `description`, printed while the step runs, and a `command`, run by the shell in the example directory.

Every example declares at least one `[[outputs]]` block. Without one there is nothing to check, and no key will take an example out of continuous integration. Unknown keys are rejected, so a typo fails the check and does not sit there unnoticed.

Record `license` as the SPDX identifier when the publisher states one. When it does not, write what the terms actually allow, as in "Free reuse with attribution to ARSO" or "Public domain (US Government)", and put the page you read them on in a comment above the key. The next person can then check the finding without repeating the search. `Unspecified` is for a publisher that genuinely states nothing, and it takes a comment saying so.

Pick the `contains` identifiers from the entities the page quotes in its closing section. A page that shows an entity the run no longer produces then fails verification, and does not quietly mislead a reader.

## Counts, and what a mismatch means

Every `[[outputs]]` block states exactly one count: `entities` when the number follows from the mapping, `at-least` when it does not. That choice is about the claim, not about the source. A fixed file's rows give an exact figure. So does a whole file that merges into a single entity. A feed whose size nobody controls admits only a floor.

What a mismatch *means* is a separate question, and `volatility` on the `[[datasets]]` blocks answers it. Against a `frozen` source, any difference is a defect and the run fails. Against a `live` one the run passes, the difference is reported, and it lands in a tracking issue without turning the matrix red. One live dataset is enough to make the whole example live: a frozen source and a live one end up in the same output file.

There is one exception. A count below half the declared figure, or above twice it, is a broken mapping whatever the publisher has done, and it fails either way. A floor has no upper rail, since a floor is deliberately set below what the feed usually returns. A missing `contains` identifier follows the same frozen and live rule as a count.

## Delivering to a broker, and running on a schedule

An example that delivers to a context broker declares where the broker is and how to start it. An example whose mapping repeats declares how long the repeating is worth watching. The runner brings the stack up, waits for the broker to serve the API, and takes it down at the end whether the mapping succeeded or failed; it interrupts a scheduled run once the cycles have had time to happen. In the container form, a broker example carries `--network host`, because the URL in the manifest is `localhost`.

```toml
[context-broker]
# The compose file, relative to the example directory. Examples that need the same broker point at
# one stack rather than each shipping a copy of it.
compose = "../22-csv-broker-temporal/docker-compose.yml"
# Where the broker answers, matching the URL the manifest delivers to.
url = "http://localhost:9090/"
# The `@context` the run's entities were written under. Only an example that asserts on a whole
# entity type needs it, and it needs it for a reason: a broker expands the type in a query against
# whichever context the query carries, so a published model's short name asked for without one
# expands against the core context and matches nothing the run wrote.
context = "https://raw.githubusercontent.com/smart-data-models/dataModel.Environment/master/context.jsonld"

# A mapping that follows a schedule polls until it is interrupted, which is what its page tells a
# reader to do by hand. The descriptor says how many polls are worth watching, and the runner
# interrupts the run once they have had time to happen. The first poll fires immediately, so two
# cycles are one interval apart; the grace covers the manifest's jitter and the last poll's work.
[schedule]
cycles = 2
interval = "5m"
grace = "2m"
```

An assertion about a broker names what to look for there in place of a file. The counts and identifiers work the same as in any other block:

```toml
# Every entity of one type the broker holds.
[[outputs]]
broker-entities = "AirQualityObserved"
at-least = 5

# One folded EntityTemporal at the broker's temporal endpoint (ETSI GS CIM 009 v1.9.1, clause 5.2.20).
[[outputs]]
broker-temporal = "urn:ngsi-ld:TropicalCyclone:AL122005"
entities = 1
contains = ["urn:ngsi-ld:TropicalCyclone:AL122005"]
```

A block names exactly one of `file`, `broker-entities`, and `broker-temporal`.

## The page

`example.md` is the example. Write it for someone who has read [Concepts](https://vela-tools.github.io/cassiopeia/guides/concepts) and nothing else. It syncs to the documentation site unchanged and has to stand on its own there, and it is the only route open to a reader without the runner. That is why its commands are checked and not trusted.

The page follows the same order every time.

1. An H1 matching `title` exactly, then an opening paragraph: what the run produces, and the single idea this example adds to the ones before it. Link back to the previous example when it sets up the problem, and forward to the next when it resolves something this one leaves open.
2. "Get the data": where the dataset comes from, who publishes it, its licence, how many records it has, and the `curl` command that downloads it, matching the `[[datasets]]` block.
3. "The record shape": one real record, quoted from the source, with the fields the mapping reads named in prose. Say outright that the mapping ignores the fields it does not name.
4. The technique, across as many sections as it takes. Identity comes first when identity is the interesting part. Quote the mapping in a `json5` block and explain the lines that are not obvious. This is the body of the page.
5. "Run it": both forms of the command, then the runner's one-step equivalent, then what the run writes. Never document only one technique. A reader with no binary installed has to be able to follow the page as written.
6. "Read the result": two or three entities from the output, quoted verbatim, with the attribute types explained against the spec wherever the shape is not self-evident. These are the entities `contains` names.

Cite the spec by clause when the output shape follows from it, for instance "ETSI GS CIM 009 v1.9.1, clause 4.5.5". Explain why an attribute has the shape it does, not only what it looks like.

Links to the guides are absolute, to `https://vela-tools.github.io/cassiopeia/`, since the guides live in another repository. Links to sibling examples are relative, `../07-csv-observed-at/example.md`, and resolve both on GitHub and on the site. Links to files in the same example directory are bare filenames.

The prose rules from the rest of the project apply: no emojis, one paragraph is one line with no hard breaks inside it, and no line references the migration, the repository split, or anything else a reader outside the project cannot see.

## The command blocks are derived

The two command blocks in the "Run it" section are generated from `[run] args`, and `cargo run -- check` fails when the page does not carry them verbatim. Write the descriptor first, run the check, and paste in whatever it says is missing.

The container form follows from the same arguments: `--user` for Docker with the Podman difference stated in prose, `--volume "$PWD:/data"`, the schemas volume only when `smart-data-models = true`, `--workdir /data`, then the image and the arguments in the order the native command uses them. The check enforces all of that, so none of it has to be remembered.

## Before the example is finished

Run it both ways from a clean directory, check the page, and regenerate the index:

```bash
rm -rf examples/NN-name/data examples/NN-name/out
cargo run -- run NN
cargo run -- run NN --runtime docker
cargo run -- check
cargo run -- index --write
```

When a new example changes the reading order, check that the pages before and after it still link correctly.
