use derive_more::{Display, FromStr};

/// The published image, used unless the caller pins another one.
const DEFAULT_IMAGE: &str = "ghcr.io/vela-tools/cassiopeia:latest";

/// A container image reference, such as `ghcr.io/vela-tools/cassiopeia:latest`.
#[derive(Debug, Clone, PartialEq, Eq, Display, FromStr)]
pub struct ContainerImage(String);

impl ContainerImage {
    /// The published image.
    pub fn published() -> ContainerImage {
        ContainerImage(DEFAULT_IMAGE.to_owned())
    }

    /// The reference as a container engine receives it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::container_image::ContainerImage;
    use std::str::FromStr;

    #[test]
    fn the_published_image_is_the_tagged_registry_reference() {
        assert_eq!(ContainerImage::published().as_str(), "ghcr.io/vela-tools/cassiopeia:latest");
    }

    #[test]
    fn a_pinned_image_is_kept_as_written() {
        let image = ContainerImage::from_str("ghcr.io/vela-tools/cassiopeia:0.4.1");

        assert_eq!(image.ok(), Some(ContainerImage("ghcr.io/vela-tools/cassiopeia:0.4.1".to_owned())));
    }
}
