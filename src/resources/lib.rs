//! # XAPI Oxidized
//! Interacts with a remote XNAT via REST exposing the **XAPI** as
//! bindings in Rust.
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

#[derive(Debug)]
pub struct ApiResource<'a, T: Display> {
    name:            &'a str,
    arg:             Option<T>,
    arg_required_by: ArgRequiredBy,
    child:           Option<Box<Self>>,
    parent:          Option<Box<Self>>,
    weight:          f32,
}

impl<'a, T: Display> ApiResource<'a, T> {
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

impl<T: Clone + Debug + Display> Iterator for ApiResource<'_, T> {
    type Item = Self;

    fn next(&mut self) -> Option<Self::Item> {
        if self.parent.is_some() {
            Some(*self.parent.as_ref().unwrap().to_owned())
        } else {
            None
        }
    }
}

pub trait PathComponent {
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

pub trait CoreResource<T> {
    fn name(&self) -> String;
    fn is_child(&self) -> bool;
    fn is_root(&self) -> bool;
    fn is_tail(&self) -> bool;
    fn required_by(&self) -> ArgRequiredBy;
    fn with_arg(&mut self, arg: T) -> &mut Self;
    fn with_arg_required(&mut self, required: ArgRequiredBy) -> &mut Self;
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

pub trait WeightedResource {
    fn weight(&self) -> f32;
}

impl<T: Display> WeightedResource for ApiResource<'_, T> {
    fn weight(&self) -> f32 {
        self.weight
    }
}

pub trait WithResource<'a, T: Display> {
    fn with_child(&mut self, child: &mut ApiResource<'a, T>) -> Result<Box<Self>>;
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
