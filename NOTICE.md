# Notice

Three licences apply here, and which one covers a file depends on what the file is. Check before you reuse anything.

The runner in `tools/runner`, its `Cargo.toml` and `Cargo.lock`, the tooling configuration (`rustfmt.toml`, `clippy.toml`, `deny.toml`, `.taplo.toml`), and the CI workflows in `.github` are under the [EUPL-1.2](LICENSE-EUPL.md), the same licence the [Cassiopeia source](https://github.com/vela-tools/cassiopeia) ships under. It is copyleft: a modified runner distributed to others goes out under the EUPL as well.

The prose is under [CC-BY-4.0](LICENSE-CC-BY.md). That covers every `example.md` page, everything in `docs/`, `README.md`, `CONTRIBUTING.md`, and this notice. Reuse it anywhere, including commercially, as long as you credit SenLab d.o.o. and say whether you changed it.

The example files are under [MIT-0](LICENSE-MIT-0.md): the mapping files, manifests, schemas, and `@context` documents (`*.json5`, `*.jsonld`), and each example's `example.toml`. They exist to be copied into your own pipeline, so there is no attribution requirement on them at all. Copy a mapping, edit it, and ship it without crediting anyone.

The datasets fall under none of the three, because no dataset is in this repository. The runner downloads each one from its publisher at run time into a gitignored `data/` directory, and the entities a run produces land in a gitignored `out/`. Both are derived from the publisher's data and stay under the publisher's terms. Each example records its dataset's origin and licence in its `example.toml`, with a note saying where the terms were read when the publisher states no SPDX identifier.

Some of those terms carry obligations that travel with the data rather than with this repository. ARSO's meteorological data must name its source. TMDB makes attribution a condition of use. Eurostat's GISCO boundaries are free to reuse non-commercially and remain the copyright of EuroGeographics, so commercial use needs a licence from them. Two datasets state no terms at all. Read the dataset's block in `example.toml` before reusing what an example downloads.

## Trademarks

Vela, Cassiopeia, and the associated logos are trademarks of SenLab d.o.o. All rights in these marks are reserved.

None of the three licences grants rights in trademarks. The EUPL-1.2 says so in Article 5, under Legal Protection: "This Licence does not grant permission to use the trade names, trademarks, service marks, or names of the Licensor, except as required for reasonable and customary use in describing the origin of the Work and reproducing the content of the copyright notice." CC-BY-4.0 says the same in Section 2(b)(2), and MIT-0 grants nothing beyond copyright either.

You may use the names to say where your software came from: "based on Cassiopeia", "a fork of Cassiopeia", "compatible with Cassiopeia". That is descriptive use and needs no permission.

You may not use the names or the logos as the name or branding of your own distribution, product, or service, and you may not use them in a way that suggests SenLab d.o.o. endorses, maintains, or is the source of your version. A modified version distributed to the public must carry a different name.

Article 5 of the EUPL, under Attribution right, is why this file travels with the code: the Licensee "shall keep intact all copyright, patent or trademarks notices" and "must include a copy of such notices and a copy of the Licence with every copy of the Work he/she distributes or communicates". CC-BY-4.0 requires the same of the prose, in Section 3(a)(1). So this notice must accompany the runner or the documentation, and any derivative of either. The MIT-0 files carry no such requirement.

Any other use of the marks needs written permission, which you can request at info@velacontext.com.
