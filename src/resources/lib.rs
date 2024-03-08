//! # URI Routes Resources.
//! A sidecar library for detailing the specifics of how a URI should
//! be constructed.
//! Allows for a rudimentary check of path arguments, when/if they are
//! required to build the resulting URI.
use std::{borrow::BorrowMut, fmt::{Debug, Display}};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug)]
pub enum ArgRequiredBy {
    Child,
    Me,
    NoOne,
    Parent,
}

impl ArgRequiredBy {
    pub fn is_child(self) -> bool {
        match self {
            ArgRequiredBy::Child => true,
            _ => false
        }
    }

    pub fn is_me(self) -> bool {
        match self {
            ArgRequiredBy::Me => true,
            _ => false
        }
    }

    pub fn is_noone(self) -> bool {
        match self {
            ArgRequiredBy::NoOne => true,
            _ => false
        }
    }

    pub fn is_parent(self) -> bool {
        match self {
            ArgRequiredBy::Parent => true,
            _ => false
        }
    }
}

#[derive(Clone, Debug)]
struct ArgNotFound(pub String);

impl Display for ArgNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "resource {:?} requires an argument", self.0)
    }
}

impl std::error::Error for ArgNotFound {}

#[derive(Clone, Debug)]
struct ParentAlreadySet(pub String);

impl Display for ParentAlreadySet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "resource {:?} parent already set", self.0)
    }
}

impl std::error::Error for ParentAlreadySet {}

#[derive(Clone, Debug)]
struct ChildAlreadySet(pub String);

impl Display for ChildAlreadySet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "resource {:?} child already set", self.0)
    }
}

impl std::error::Error for ChildAlreadySet {}

/// Represents a single part of of a URI path.
/// Where arguments are optional, there are
/// interfaces which allow this object to check
/// if an argument is required by either this
/// component, or entities that are related to it.
#[derive(Debug)]
pub struct ApiResource<'a, T: Display> {
    name:            &'a str,
    arg:             Option<T>,
    arg_required_by: ArgRequiredBy,
    child:           Option<Box<Self>>,
    parent:          Option<Box<Self>>,
    weight:          f32,
}

/// Barebones basic implementation of an
/// `ApiResource`.
/// ```rust
/// use uri_resources::ApiResource;
/// let resource: ApiResource<'_, String> = ApiResource::new("resource");
/// ```
impl<'a, T: Display> ApiResource<'a, T> {
    /// Create a new instance of `ApiResource`.
    pub fn new<'b: 'a>(name: &'b str) -> Self {
        Self{
            name: name,
            arg: None,
            arg_required_by: ArgRequiredBy::NoOne,
            child: None,
            parent: None,
            weight: 0.0
        }
    }
}

impl<T: Clone + Display> Clone for ApiResource<'_, T> {
    fn clone(&self) -> Self {
        Self{
            name: self.name,
            arg:  self.arg.clone(),
            arg_required_by: self.arg_required_by,
            child: self.child.clone(),
            parent: self.parent.clone(),
            weight: self.weight
        }
    }
}

/// Composes an object into a path component,
/// conditionally failing if the implemented
/// instance does not meet the requirements set
/// by it's declaration.
///
/// Ensure resources can be digested as path
/// components.
/// ```rust
/// use uri_resources::{ApiResource, PathComponent};
/// let path = ApiResource::<String>::new("resource").as_path_component();
/// assert!(!path.is_err())
/// ```
///
/// Ensure resources can be digested and return
/// the expected value.
/// ```rust
/// use uri_resources::{ApiResource, PathComponent};
/// let path = ApiResource::<String>::new("resource").as_path_component();
/// assert_eq!(path.unwrap(), String::from("resource/"))
/// ```
pub trait PathComponent {
    /// Composes this as a path component.
    fn as_path_component(&self) -> Result<String>;
}

impl<'a, T: Debug + Display + Clone> PathComponent for ApiResource<'a, T> {
    fn as_path_component(&self) -> Result<String> {
        if self.arg_required_by.is_me() && self.arg.is_none() {
            Err(ArgNotFound(self.name().to_owned()).into())
        } else if self.arg_required_by.is_parent() && self.parent.is_some() && self.arg.is_none() {
            Err(ArgNotFound(self
                .parent
                .as_ref()
                .unwrap()
                .name()
                .to_owned()).into())
        } else if self.arg_required_by.is_child() && self.child.is_some() && self.arg.is_none() {
            Err(ArgNotFound(self
                .child
                .as_ref()
                .unwrap()
                .name()
                .to_owned()).into())
        } else {
            Ok(format!("{}/{}", self.name(), self.arg.clone().map_or("".into(), |a| a.to_string())))
        }
    }
}

pub trait ArgedResource<T> {
    fn argument(&self) -> Option<&T>;
}

impl<'a, T: Clone + Display> ArgedResource<T> for ApiResource<'a, T> {
    fn argument(&self) -> Option<&T> {
        self.arg.as_ref()
    }
}

/// The core functionality that is to be expected
/// of some resource object. These methods assist
/// in the work done by other traits in this
/// library. Specifically by managing the the
/// resource and it's relatives.
pub trait CoreResource<T> {
    /// The name of the resource component. Is
    /// used as the path component on digestion.
    fn name(&self) -> String;
    /// If this is a child of another resource.
    ///
    /// Initialy created object should produce a
    /// non-child node.
    /// ```rust
    /// use uri_resources::{ApiResource, CoreResource};
    /// let resource = ApiResource::<String>::new("resource");
    /// assert_eq!(resource.is_child(), false)
    /// ```
    ///
    /// Try to create an instance of two nodes
    /// where one is related to the other as the
    /// parent.
    /// ```rust
    /// use uri_resources::{ApiResource, CoreResource, WithResource};
    /// let mut child = ApiResource::<String>::new("child_resource");
    /// let parent = ApiResource::<String>::new("parent_resource")
    ///     .with_child(&mut child);
    /// assert_eq!(child.is_child(), true)
    /// ```
    fn is_child(&self) -> bool;
    /// If this is the first resource of the path.
    ///
    /// Initialy created object should produce a
    /// root node.
    /// ```rust
    /// use uri_resources::{ApiResource, CoreResource};
    /// let resource = ApiResource::<String>::new("resource");
    /// assert_eq!(resource.is_root(), true)
    /// ```
    ///
    /// Subsequent objects should not be a root
    /// node.
    /// ```rust
    /// use uri_resources::{ApiResource, CoreResource, WithResource};
    /// let mut child = ApiResource::<String>::new("child_resource");
    /// let parent = ApiResource::<String>::new("parent_resource")
    ///     .with_child(&mut child);
    /// assert_ne!(child.is_root(), true)
    /// ```
    fn is_root(&self) -> bool;
    /// If this is the last resource of the path.
    ///
    /// Root node can be a tail node if it is the
    /// only resource node.
    /// ```rust
    /// use uri_resources::{ApiResource, CoreResource};
    /// let resource = ApiResource::<String>::new("resource");
    /// assert!(resource.is_tail())
    /// ```
    ///
    /// If there are otherwise child nodes, a root
    /// node cannot be the 'tail'.
    /// ```
    /// use uri_resources::{ApiResource, CoreResource, WithResource};
    /// let mut child0 = ApiResource::<String>::new("child_resource0");
    /// let mut child1 = ApiResource::<String>::new("child_resource1");
    ///
    /// child0 = *child0.with_child(&mut child1).expect("resource node");
    /// let parent = ApiResource::<String>::new("parent_resource")
    ///     .with_child(&mut child0);
    /// assert!(!parent.expect("parent node").is_tail())
    /// ```
    ///
    /// The middle child cannot be the tail.
    /// ```rust
    /// use uri_resources::{ApiResource, CoreResource, WithResource};
    /// let mut child0 = ApiResource::<String>::new("child_resource0");
    /// let mut child1 = ApiResource::<String>::new("child_resource1");
    ///
    /// child0 = *child0.with_child(&mut child1).expect("resource node");
    /// let parent = ApiResource::<String>::new("parent_resource")
    ///     .with_child(&mut child0);
    /// assert!(child0.is_child() && !child0.is_tail());
    /// ```
    ///
    /// The last child should be the tail.
    /// ```rust
    /// use uri_resources::{ApiResource, CoreResource, WithResource};
    /// let mut child0 = ApiResource::<String>::new("child_resource0");
    /// let mut child1 = ApiResource::<String>::new("child_resource1");
    ///
    /// child0 = *child0.with_child(&mut child1).expect("resource node");
    /// let parent = ApiResource::<String>::new("parent_resource")
    ///     .with_child(&mut child0);
    /// assert!(child1.is_child() && child1.is_tail())
    /// ```
    fn is_tail(&self) -> bool;
    /// Determines if, and by whom, an argument
    /// set on this is required.
    fn required_by(&self) -> ArgRequiredBy;
    /// Sets an argument on this resource
    /// component.
    fn with_arg(&mut self, arg: T) -> &mut Self;
    /// Sets if, and by whom, this component's
    /// argument is required.
    fn with_arg_required(&mut self, required: ArgRequiredBy) -> &mut Self;
    /// Determines the ordering weight to be used
    /// by pre-digestion sorting.
    fn with_weight(&mut self, weight: f32) -> &mut Self;
}

impl<'a, T: Clone + Display> CoreResource<T> for ApiResource<'a, T> {
    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn is_child(&self) -> bool {
        self.parent.is_some()
    }

    fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    fn is_tail(&self) -> bool {
        self.child.is_none()
    }

    fn required_by(&self) -> ArgRequiredBy {
        self.arg_required_by
    }

    fn with_arg(&mut self, arg: T) -> &mut Self {
        self.arg = Some(arg);
        self
    }

    fn with_arg_required(&mut self, required: ArgRequiredBy) -> &mut Self {
        self.arg_required_by = required;
        self
    }

    fn with_weight(&mut self, weight: f32) -> &mut Self {
        self.weight = weight;
        self
    }
}

/// Resource can be 'weighted'. This allows use
/// in `uri_routes`, after digestion to sort
/// paths in the final required. order.
pub trait WeightedResource {
    /// The sorting weight value of this.
    fn weight(&self) -> f32;
}

impl<T: Display> WeightedResource for ApiResource<'_, T> {
    fn weight(&self) -> f32 {
        self.weight
    }
}

/// Allows resources to set their child and parent
/// nodes.
pub trait WithResource<'a, T: Display> {
    // Not adding a test here as prior tests cover
    // this well enough.

    /// Adds a child node to this resource. Fails
    /// if the child is already set.
    fn with_child(&mut self, child: &mut ApiResource<'a, T>) -> Result<Box<Self>>;
    /// Adds the parent node to this resource.
    /// Fails if the parent is already set.
    fn with_parent(&mut self, parent: &mut ApiResource<'a, T>) -> Result<Box<Self>>;
}

impl<'a, T: Debug + Display + Clone> WithResource<'a, T> for ApiResource<'a, T> {
    fn with_child(&mut self, child: &mut ApiResource<'a, T>) -> Result<Box<Self>> {
        match self.child {
            None => {
                let mut new = self.clone();
                match child.with_parent(new.borrow_mut()) {
                    Ok(chld) => {
                        new.child = Some(Box::new(chld.as_ref().clone()));
                        Ok(Box::new(new))
                    },
                    Err(e) => Err(e)
                }
            },
            Some(_) => Err(ChildAlreadySet(self.name().to_owned()).into())
        }
    }

    fn with_parent(&mut self, parent: &mut ApiResource<'a, T>) -> Result<Box<Self>> {
        match self.parent {
            None => {
                self.parent = Box::new(parent.clone()).into();
                Ok(Box::new(self.clone()))
            },
            Some(_) => Err(ParentAlreadySet(self.name().to_owned()).into())
        }
    }
}
