use crate::source_format::SourceFormat;
use derive_more::Display;

/// The shields.io endpoint every badge is built from.
const SHIELDS: &str = "https://img.shields.io/badge";

/// The left half every badge carries, so an entry reads as a labelled field rather than a bare tag.
const LABEL: &str = "format";

/// A source format as the coloured badge the README index tags an example with.
///
/// The colour belongs to the format rather than to the example, so the index is scannable by input:
/// every CSV example carries the same swatch. The badges are the same shape as the ones in the
/// README header, which is why no style is asked for: shields.io's default is what the rest uses.
///
/// The badge is written as an `<img>` rather than as a markdown image because it is floated to the
/// right edge of the entry, which markdown has no syntax for: `align="right"` is the presentational
/// attribute browsers map to `float: right`, and it sidesteps aligning a badge against text
/// altogether. It is emitted before the title so the float is placed before the line is laid out.
#[derive(Debug, Clone, Copy, Display)]
#[display(r#"<img src="{SHIELDS}/{LABEL}-{}-{}" alt="{LABEL}: {_0}" height="20" align="right">"#, escaped(&_0.to_string()), colour(*_0))]
pub struct FormatBadge(SourceFormat);

impl From<SourceFormat> for FormatBadge {
    fn from(format: SourceFormat) -> FormatBadge {
        FormatBadge(format)
    }
}

/// The badge text as shields.io needs it written.
///
/// A dash separates the message from the colour in a shields.io path, so a dash inside the message
/// is doubled to survive the split.
fn escaped(token: &str) -> String {
    token.replace('-', "--")
}

/// The colour a source format is tagged with.
const fn colour(format: SourceFormat) -> &'static str {
    match format {
        SourceFormat::Csv => "1d6f42",
        SourceFormat::Json => "d97706",
        SourceFormat::GeoJson => "2563eb",
        SourceFormat::Kml => "7c3aed",
        SourceFormat::Kmz => "9333ea",
        SourceFormat::Xml => "db2777",
        SourceFormat::Shapefile => "92400e",
        SourceFormat::Grib1 => "0f766e",
        SourceFormat::Grib2 => "0d9488",
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        format_badge::{FormatBadge, escaped},
        source_format::SourceFormat,
    };
    use strum::IntoEnumIterator;

    #[test]
    fn a_badge_labels_the_format_and_carries_it_as_the_message() {
        let badge = FormatBadge::from(SourceFormat::Csv).to_string();

        assert_eq!(
            badge,
            r#"<img src="https://img.shields.io/badge/format-csv-1d6f42" alt="format: csv" height="20" align="right">"#
        );
    }

    #[test]
    fn a_dash_in_a_token_is_doubled_so_shields_io_keeps_it() {
        assert_eq!(escaped("geo-json"), "geo--json");
    }

    #[test]
    fn a_multi_word_format_keeps_the_token_cassiopeia_takes() {
        let badge = FormatBadge::from(SourceFormat::GeoJson).to_string();

        assert_eq!(
            badge,
            r#"<img src="https://img.shields.io/badge/format-geojson-2563eb" alt="format: geojson" height="20" align="right">"#
        );
    }

    #[test]
    fn every_format_is_tagged_with_a_colour_of_its_own() {
        let mut colours: Vec<String> = SourceFormat::iter().map(|format| FormatBadge::from(format).to_string()).collect();
        let count = colours.len();
        colours.sort();
        colours.dedup();

        assert_eq!(colours.len(), count);
    }
}
