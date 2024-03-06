use http::uri;
use ordered_float::OrderedFloat;

/// Constructs URL routes from the ground up.
/// Useful in scenarios where the need to
/// dynamically construct routes that may have
/// common properties.
pub trait RouteBuilder<'a> {
    /// New instance of a `RouteBuilder`.
    fn new(host: &'a str) -> Self;
    /// Tries to build a URI from path arguments
    /// and parameters.
    fn build(self) -> Result<uri::Uri, http::Error>;
    /// Add a parameter key/pair to the builder.
    fn with_param<T: ToString>(self, name: &'a str, value: T) -> Self;
    /// Add a path argument to the end of the
    /// path buffer.
    fn with_path(self, path: &'a str) -> Self;
    /// Inserts a path argument with the desired
    /// weight.
    fn with_path_weight(self, path: &'a str, weight: f32) -> Self;
    /// Set the protocol scheme.
    fn with_scheme(self, scheme: &'a str) -> Self;
}

#[derive(Clone, Eq, Ord)]
struct ApiRoutePath {
    path:   String,
    weight: OrderedFloat<f32>,
}

impl ApiRoutePath {
    pub fn new<'a>(path: &'a str, weight: f32) -> Self {
        Self{path: path.to_owned(), weight: OrderedFloat::from(weight)}
    }
}

impl PartialEq for ApiRoutePath {
    fn eq(&self, other: &Self) -> bool {
        self.weight == other.weight && self.path == other.path
    }
}

impl PartialEq<str> for ApiRoutePath {
    fn eq(&self, other: &str) -> bool {
        self.path == other
    }
}

impl PartialOrd for ApiRoutePath {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.weight.partial_cmp(&other.weight)
    }
}

impl ToString for ApiRoutePath {
    fn to_string(&self) -> String {
        self.path.clone()
    }
}

pub struct ApiRouteBuilder<'a> {
    hostname:   &'a str,
    parameters: Vec<String>,
    scheme:     Option<String>,
    sub_paths:  Vec<ApiRoutePath>,
}

impl<'a> ApiRouteBuilder<'a> {
    fn insert_param<T: ToString>(mut self, name: &'a str, value: T) -> Self {
        self.parameters.push(format!("{name}={}", value.to_string()));
        self
    }

    fn insert_path(mut self, path: &'a str, weight: Option<f32>) -> Self {
        let weight = weight
            .unwrap_or(f32::MAX)
            .clamp(0.1, f32::MAX);
        let path = ApiRoutePath::new(path, weight);
        self.sub_paths.push(path);
        self.sub_paths.sort();
        self
    }

    fn insert_scheme(mut self, scheme: Option<String>) -> Self {
        self.scheme = scheme;
        self
    }

    fn parse_params(&self) -> String {
        self.parameters.join("&")
    }

    fn parse_path(&self) -> String {
        let mut paths = self.sub_paths.clone();
        paths.retain(|p| p != "");

        let paths: Vec<_> = paths
            .iter()
            .map(|p| p.to_string())
            .collect();
        paths.join("/").replace("//", "/")
    }

    fn parse_scheme(&self) -> String {
        self.scheme.clone().unwrap_or(String::from("https"))
    }
}

impl<'a> RouteBuilder<'a> for ApiRouteBuilder<'a> {
    fn new(host: &'a str) -> Self {
        Self{
            hostname: host,
            parameters: vec![],
            scheme: None,
            sub_paths: vec![ApiRoutePath::new("/", 0.0)]
        }
    }

    /// Tries to build a URI from path arguments
    /// and parameters.
    /// ```rust
    /// use crate::uri_routes::{RouteBuilder, ApiRouteBuilder};
    /// let route = ApiRouteBuilder::new("google.com").build().unwrap();
    /// assert_eq!(route, "https://google.com")
    /// ```
    fn build(self) -> Result<uri::Uri, http::Error> {
        let scheme   = self.parse_scheme();
        let hostname = self.hostname;
        let path     = self.parse_path();
        let params   = self.parse_params();

        uri::Builder::new()
            .scheme(scheme.as_str())
            .authority(hostname)
            .path_and_query(format!("{path}?{params}"))
            .build()
    }

    /// Add a parameter key/pair to the builder.
    /// ```rust
    /// use crate::uri_routes::{RouteBuilder, ApiRouteBuilder};
    /// let route = ApiRouteBuilder::new("fqdm.org")
    ///     .with_param("page", 1)
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(route, "https://fqdm.org?page=1")
    /// ```
    fn with_param<T: ToString>(self, name: &'a str, value: T) -> Self {
        self.insert_param(name, value)
    }

    /// Add a path argument to the end of the
    /// path buffer.
    /// ```rust
    /// use crate::uri_routes::{RouteBuilder, ApiRouteBuilder};
    /// let route = ApiRouteBuilder::new("fqdm.org")
    ///     .with_path("resource")
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(route, "https://fqdm.org/resource")
    /// ```
    fn with_path(self, path: &'a str) -> Self {
        self.insert_path(path, None)
    }

    /// Inserts a path argument with the desired
    /// weight.
    /// ```rust
    /// use crate::uri_routes::{RouteBuilder, ApiRouteBuilder};
    /// let route = ApiRouteBuilder::new("fqdm.org")
    ///     .with_path_weight("resource0", 2.0)
    ///     .with_path_weight("resource1", 1.0)
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(route, "https://fqdm.org/resource1/resource0")
    /// ```
    fn with_path_weight(self, path: &'a str, weight: f32) -> Self {
        self.insert_path(path, Some(weight))
    }

    /// Tries to build a URI from path arguments
    /// and parameters.
    /// ```rust
    /// use crate::uri_routes::{RouteBuilder, ApiRouteBuilder};
    /// let route = ApiRouteBuilder::new("localhost")
    ///     .with_scheme("file")
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(route, "file://localhost")
    /// ```
    fn with_scheme(self, scheme: &'a str) -> Self {
        self.insert_scheme(Some(scheme.to_owned()))
    }
}
