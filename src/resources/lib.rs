//! # URI Routes Resources.
//! A sidecar library for detailing the specifics of how a URI should
//! be constructed.
//! Allows for a rudimentary check of path arguments, when/if they are
//! required to build the resulting URI.
use std::{borrow::BorrowMut, fmt::{Debug, Display}};

use anyhow::Result;

#[derive(Clone, Copy, Debug)]
pub enum ArgRequiredBy {
    Child,
    Me,
    NoOne,
    Parent,
}

impl ArgRequiredBy {
    pub fn is_child(self) -> bool {
        matches!(self, Self::Child)
    }

    pub fn is_me(self) -> bool {
        matches!(self, Self::Me)
    }

    pub fn is_noone(self) -> bool {
        matches!(self, Self::NoOne)
    }

    pub fn is_parent(self) -> bool {
        matches!(self, Self::Parent)
    }
}

#[derive(thiserror::Error, Clone, Debug)]
pub enum ArgError {
    #[error("{0} requires an argument")]
    Missing(String),
    #[error("{0} invalid with reason(s): {1:?}")]
    NotValid(String, Vec<String>)
}

#[derive(thiserror::Error, Clone, Debug)]
pub enum ResourceError {
    #[error("existing {1} node of {0} already set")]
    AlreadySet(String, String),
}

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
    arg_validators:  Vec<fn(&T) -> Result<()>>,
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
            name,
            arg: None,
            arg_required_by: ArgRequiredBy::NoOne,
            arg_validators: vec![],
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
            arg_validators: self.arg_validators.clone(),
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
pub trait PathComponent {
    /// Composes this as a path component.
    ///
    /// Ensure resources can be digested and return
    /// the expected value.
    /// ```rust
    /// use uri_resources::{ApiResource, PathComponent};
    /// let path = ApiResource::<String>::new("resource").as_path_component();
    /// assert_eq!(path.unwrap(), String::from("resource/"))
    /// ```
    fn as_path_component(&self) -> Result<String>;
    /// Compose the entire heirarchy of components
    /// into one string.
    ///
    /// Ensure the composition of a multi node
    /// collection can be composed into a single
    /// String value without error.
    /// ```rust
    /// use uri_resources::{ApiResource, LinkedResource, PathComponent};
    /// let mut child0 = ApiResource::<String>::new("child_resource0");
    /// let mut child1 = ApiResource::<String>::new("child_resource1");
    ///
    /// child0 = *child0.with_child(&mut child1).expect("resource node");
    /// let parent = ApiResource::<String>::new("parent_resource")
    ///     .with_child(&mut child0);
    ///
    /// let path = parent.expect("parent node").compose();
    /// assert!(!path.is_err())
    /// ```
    ///
    /// Ensure the composition of a multi node
    /// collection can be composed into a single
    /// String value without error.
    /// ```rust
    /// use uri_resources::{ApiResource, LinkedResource, PathComponent};
    /// let mut child0 = ApiResource::<String>::new("child_resource0");
    /// let mut child1 = ApiResource::<String>::new("child_resource1");
    ///
    /// child0 = *child0.with_child(&mut child1).expect("resource node");
    /// let parent = ApiResource::<String>::new("parent_resource")
    ///     .with_child(&mut child0);
    ///
    /// let path = parent.expect("parent node").compose();
    /// assert_eq!(path.expect("composed path"), "parent_resource/child_resource0/child_resource1/")
    /// ```
    fn compose(&self) -> Result<String>;
}

impl<'a, T: Debug + Display + Clone> PathComponent for ApiResource<'a, T> {
    fn as_path_component(&self) -> Result<String> {
        let to_argnotfound = |n: &Self| {
            Err(ArgError::Missing(n.name().to_owned()).into())
        };

        let compose_this = || {
            let errors: Vec<_> = self.arg_validators
                .iter()
                .map(|f| { (f)(self.arg.as_ref().unwrap()) })
                .filter(|r| r.is_err())
                .map(|r| r.unwrap_err().to_string())
                .collect();

            if !errors.is_empty()  {
                Err(ArgError::NotValid(self.name(), errors).into())
            } else {
                let ret = format!(
                    "{}/{}",
                    self.name(),
                    self.arg.clone().map_or("".into(), |a| a.to_string()));
                Ok(ret)
            }
        };

        if self.arg.is_some() || self.required_by().is_noone() {
            compose_this()
        } else if self.required_by().is_parent() && self.parent.is_some() {
            to_argnotfound(self.parent().unwrap())
        } else if self.required_by().is_child() && self.child.is_some() {
            to_argnotfound(self.child().unwrap())
        } else {
            compose_this()
        }
    }

    fn compose(&self) -> Result<String> {
        let mut curr = Some(self);
        let mut components = vec![];

        while curr.is_some() {
            components.push(match curr.unwrap().as_path_component() {
                Ok(path) => {
                    curr = curr.unwrap().child();
                    path
                },
                e => return e
            });
        }
        Ok(components.join("/").replace("//", "/"))
    }
}

pub trait ArgedResource<T> {
    /// Argument set on this resource.
    fn argument(&self) -> Option<&T>;
    /// Determines if, and by whom, an argument
    /// set on this is required.
    fn required_by(&self) -> ArgRequiredBy;
    /// Sets an argument on this resource
    /// component.
    fn with_arg(&mut self, arg: T) -> &mut Self;
    /// Sets if, and by whom, this component's
    /// argument is required.
    fn with_arg_required(&mut self, required: ArgRequiredBy) -> &mut Self;
}

impl<'a, T: Clone + Display> ArgedResource<T> for ApiResource<'a, T> {
    fn argument(&self) -> Option<&T> {
        self.arg.as_ref()
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
}

impl<'a, T: Clone + Display> CoreResource<T> for ApiResource<'a, T> {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}

/// Allows resources to set their child and parent
/// nodes.
pub trait LinkedResource<'a, T: Display> {
    /// The child `Resource` node.
    fn child(&self) -> Option<&Self>;
    /// The parent `Resource` node.
    fn parent(&self) -> Option<&Self>;
    /// If this is a child of another resource.
    ///
    /// Initialy created object should produce a
    /// non-child node.
    /// ```rust
    /// use uri_resources::{ApiResource, LinkedResource};
    /// let resource = ApiResource::<String>::new("resource");
    /// assert_eq!(resource.is_child(), false)
    /// ```
    ///
    /// Try to create an instance of two nodes
    /// where one is related to the other as the
    /// parent.
    /// ```rust
    /// use uri_resources::{ApiResource, LinkedResource};
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
    /// use uri_resources::{ApiResource, LinkedResource};
    /// let resource = ApiResource::<String>::new("resource");
    /// assert_eq!(resource.is_root(), true)
    /// ```
    ///
    /// Subsequent objects should not be a root
    /// node.
    /// ```rust
    /// use uri_resources::{ApiResource, LinkedResource};
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
    /// use uri_resources::{ApiResource, LinkedResource};
    /// let resource = ApiResource::<String>::new("resource");
    /// assert!(resource.is_tail())
    /// ```
    ///
    /// If there are otherwise child nodes, a root
    /// node cannot be the 'tail'.
    /// ```
    /// use uri_resources::{ApiResource, LinkedResource};
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
    /// use uri_resources::{ApiResource, LinkedResource};
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
    /// use uri_resources::{ApiResource, LinkedResource};
    /// let mut child0 = ApiResource::<String>::new("child_resource0");
    /// let mut child1 = ApiResource::<String>::new("child_resource1");
    ///
    /// child0 = *child0.with_child(&mut child1).expect("resource node");
    /// let parent = ApiResource::<String>::new("parent_resource")
    ///     .with_child(&mut child0);
    /// assert!(child1.is_child() && child1.is_tail())
    /// ```
    fn is_tail(&self) -> bool;
    /// Adds a child node to this resource. Fails
    /// if the child is already set.
    fn with_child(&mut self, child: &mut ApiResource<'a, T>) -> Result<Box<Self>>;
    /// Adds the parent node to this resource.
    /// Fails if the parent is already set.
    fn with_parent(&mut self, parent: &mut ApiResource<'a, T>) -> Result<Box<Self>>;
}

impl<'a, T: Debug + Display + Clone> LinkedResource<'a, T> for ApiResource<'a, T> {
    fn child(&self) -> Option<&Self> {
        self.child.as_deref()
    }

    fn parent(&self) -> Option<&Self> {
        self.parent.as_deref()
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
            Some(_) => Err(ResourceError::AlreadySet(self.name(), "child".into()).into())
        }
    }

    fn with_parent(&mut self, parent: &mut ApiResource<'a, T>) -> Result<Box<Self>> {
        match self.parent {
            None => {
                self.parent = Box::new(parent.clone()).into();
                Ok(Box::new(self.clone()))
            },
            Some(_) => Err(ResourceError::AlreadySet(self.name(), "parent".into()).into())
        }
    }
}

/// Resource can be 'weighted'. This allows use
/// in `uri_routes`, after digestion to sort
/// paths in the final required. order.
pub trait WeightedResource {
    /// The sorting weight value of this.
    fn weight(&self) -> f32;
    /// Determines the ordering weight to be used
    /// by pre-digestion sorting.
    fn with_weight(&mut self, weight: f32) -> &Self;
}

impl<T: Display> WeightedResource for ApiResource<'_, T> {
    fn weight(&self) -> f32 {
        self.weight
    }

    fn with_weight(&mut self, weight: f32) -> &Self {
        self.weight = weight;
        self
    }
}

pub trait Resource<'a, T: Clone + Display>:
    CoreResource<T> +
    ArgedResource<T> +
    LinkedResource<'a, T> +
    WeightedResource {}

impl<'a, T: Clone + Debug + Display> Resource<'a, T> for ApiResource<'a, T> {}
