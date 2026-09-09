use strum::Display;

/// Whether a container run shares the host's network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum ContainerNetwork {
    /// The engine's own network. Nothing published on the host is reachable from inside the
    /// container, which is what every run that writes files wants.
    #[default]
    Isolated,

    /// The host's network, so a broker published on a host port answers at the same `localhost`
    /// URL inside the container as it does outside it. A run that delivers to a broker needs this,
    /// because the URL its manifest names is the one its page tells a reader to query by hand.
    Host,
}

#[cfg(test)]
mod tests {
    use crate::container_network::ContainerNetwork;

    #[test]
    fn a_run_that_needs_no_broker_is_isolated_by_default() {
        assert_eq!(ContainerNetwork::default(), ContainerNetwork::Isolated);
    }

    #[test]
    fn host_networking_prints_the_value_a_container_engine_takes() {
        assert_eq!(ContainerNetwork::Host.to_string(), "host");
    }
}
