use crate::cassiopeia_release::CassiopeiaRelease;
use derive_more::{Display, FromStr};

/// Where every Cassiopeia release publishes its image, tagged with the release's own name.
const RELEASE_REPOSITORY: &str = "ghcr.io/vela-tools/cassiopeia";

/// A container image reference, such as `ghcr.io/vela-tools/cassiopeia:v1.0.3`.
#[derive(Debug, Clone, PartialEq, Eq, Display, FromStr)]
pub struct ContainerImage(String);

impl ContainerImage {
    /// The image a release published.
    pub fn release(release: &CassiopeiaRelease) -> ContainerImage {
        ContainerImage(format!("{RELEASE_REPOSITORY}:{release}"))
    }

    /// The reference as a container engine receives it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::{cassiopeia_release::CassiopeiaRelease, container_image::ContainerImage};
    use semver::Version;
    use std::str::FromStr;

    #[test]
    fn a_release_image_is_tagged_with_the_release() {
        let image = ContainerImage::release(&CassiopeiaRelease::from(Version::new(1, 0, 3)));

        assert_eq!(image.as_str(), "ghcr.io/vela-tools/cassiopeia:v1.0.3");
    }

    #[test]
    fn a_pre_release_image_is_tagged_with_the_pre_release() {
        let image = CassiopeiaRelease::from_str("v1.1.0-alpha1").map(|release| ContainerImage::release(&release));

        assert_eq!(image.ok(), Some(ContainerImage("ghcr.io/vela-tools/cassiopeia:v1.1.0-alpha1".to_owned())));
    }

    #[test]
    fn a_pinned_image_is_kept_as_written() {
        let image = ContainerImage::from_str("ghcr.io/vela-tools/cassiopeia:0.4.1");

        assert_eq!(image.ok(), Some(ContainerImage("ghcr.io/vela-tools/cassiopeia:0.4.1".to_owned())));
    }
}
