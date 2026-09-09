use serde::Deserialize;
use strum::{Display, EnumIter};

/// A source format an example ingests.
///
/// The variants name formats Cassiopeia reads, and the serialised form is the token the CLI's
/// `--type` flag and a manifest's `format` field use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Display, EnumIter)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum SourceFormat {
    /// Delimited text, with or without a header row.
    Csv,
    /// A JSON document, typically a top-level array of records.
    Json,
    /// `GeoJSON` features, each with a geometry.
    ///
    /// Kebab-case would write this `geo-json`, which Cassiopeia accepts only as a legacy alias. The
    /// token the `--type` flag and a manifest's `format` field take is `geojson`, so that is what a
    /// descriptor declares and what the index shows.
    #[serde(rename = "geojson")]
    #[strum(serialize = "geojson")]
    GeoJson,
    /// KML placemarks and folders.
    Kml,
    /// A zipped KML archive.
    Kmz,
    /// An XML document.
    Xml,
    /// A zipped ESRI shapefile.
    Shapefile,
    /// GRIB edition 1, which needs ecCodes on the host.
    Grib1,
    /// GRIB edition 2.
    Grib2,
}

#[cfg(test)]
mod tests {
    use crate::source_format::SourceFormat;
    use serde::Deserialize;
    use strum::IntoEnumIterator;

    /// The shape a format list is read in, since a bare enum is not a TOML document.
    #[derive(Debug, Deserialize)]
    struct Declaration {
        formats: Vec<SourceFormat>,
    }

    #[test]
    fn geojson_is_read_and_printed_as_cassiopeia_writes_it() {
        let declaration: Result<Declaration, _> = toml::from_str("formats = [\"geojson\"]\n");

        assert_eq!(declaration.map(|declaration| declaration.formats).ok(), Some(vec![SourceFormat::GeoJson]));
        assert_eq!(SourceFormat::GeoJson.to_string(), "geojson");
    }

    #[test]
    fn the_legacy_hyphenated_geojson_token_is_rejected() {
        let declaration: Result<Declaration, _> = toml::from_str("formats = [\"geo-json\"]\n");

        assert!(declaration.is_err());
    }

    #[test]
    fn every_format_reads_back_the_token_it_prints() {
        for format in SourceFormat::iter() {
            let text = format!("formats = [\"{format}\"]\n");
            let declaration: Result<Declaration, _> = toml::from_str(&text);

            assert_eq!(declaration.map(|declaration| declaration.formats).ok(), Some(vec![format]));
        }
    }

    #[test]
    fn a_format_cassiopeia_does_not_read_is_rejected() {
        let declaration: Result<Declaration, _> = toml::from_str("formats = [\"parquet\"]\n");

        assert!(declaration.is_err());
    }
}
